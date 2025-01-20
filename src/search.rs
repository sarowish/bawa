use crate::{
    app::StatefulList,
    input::{handle_key_fuzzy_mode, Input},
    profile::Profile,
    ui, utils,
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32String,
};
use std::fmt::Display;

#[derive(Default)]
pub struct Search {
    pub matches: Vec<usize>,
    pub pattern: String,
}

impl Search {
    pub fn new(pattern: &str) -> Self {
        Self {
            matches: Vec::new(),
            pattern: pattern.to_string().to_lowercase(),
        }
    }

    pub fn search<T: Display>(&mut self, list: &[T]) {
        if self.pattern.is_empty() {
            return;
        }

        self.matches = list
            .iter()
            .enumerate()
            .filter(|(_, text)| text.to_string().to_lowercase().contains(&self.pattern))
            .map(|(idx, _)| idx)
            .collect();
    }
}

#[derive(PartialEq, Eq)]
pub struct MatchedItem {
    entry: Entry,
    pub text: String,
    score: Option<u32>,
    indices: Vec<u32>,
}

impl MatchedItem {
    fn new(entry: Entry, score: Option<u32>, indices: &[u32]) -> Self {
        Self {
            text: entry.to_string(),
            entry,
            score,
            indices: Vec::from(indices),
        }
    }

    pub fn path(&self) -> String {
        self.entry.path.to_string()
    }

    pub fn profile(&self) -> String {
        self.entry.profile_name.to_string()
    }

    pub fn highlight_slices(&self) -> Vec<(&str, bool)> {
        if self.indices.is_empty() {
            return vec![(&self.text, false)];
        }

        let text = self.text.as_str();
        let mut slices = Vec::new();
        let mut highlighted_indices = vec![false; text.len()];

        for idx in &self.indices {
            highlighted_indices[*idx as usize] = true;
        }

        let mut prev_idx = 0;
        let upper_boundary_map = utils::upper_char_boundaries(text);

        for (idx, pair) in highlighted_indices.windows(2).enumerate() {
            if pair[0] != pair[1] {
                let idx = upper_boundary_map[idx];
                slices.push((&text[prev_idx..idx], pair[0]));
                prev_idx = idx;
            }
        }

        if prev_idx != text.len() {
            slices.push((&text[prev_idx..], highlighted_indices[prev_idx]));
        }

        slices
    }
}

#[derive(Clone, Eq, PartialEq)]
struct Entry {
    profile_name: Utf32String,
    path: Utf32String,
}

impl Entry {
    fn new(profile: &Profile, path: &str) -> Self {
        Self {
            profile_name: Utf32String::from(profile.name.clone()),
            path: Utf32String::from(path),
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.profile_name.is_empty() {
            write!(f, "{}", self.path)
        } else {
            write!(f, "{}    {}", self.profile_name, self.path)
        }
    }
}

pub struct FuzzyFinder {
    matcher: Matcher,
    pub input: Input,
    pattern: Pattern,
    entries: Vec<Entry>,
    pub matched_items: StatefulList<MatchedItem>,
    pub total_count: usize,
    pub match_count: usize,
}

impl FuzzyFinder {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            input: Input::with_prompt("> "),
            pattern: Pattern::default(),
            entries: Vec::new(),
            matched_items: StatefulList::with_items(Vec::new()),
            total_count: 0,
            match_count: 0,
        }
    }

    pub fn fill_paths(&mut self, profile: &Profile) {
        self.entries.append(
            &mut profile
                .get_file_rel_paths(false)
                .iter()
                .map(|path| Entry::new(profile, path))
                .collect(),
        );
        self.total_count = self.entries.len();
    }

    pub fn clean_profile(&mut self) {
        for entry in &mut self.entries {
            entry.profile_name = Utf32String::default();
        }
    }

    pub fn reset(&mut self) {
        self.input.set_text("");
        self.entries.drain(..);
    }

    pub fn update_matches(&mut self) {
        self.pattern
            .reparse(&self.input.text, CaseMatching::Smart, Normalization::Smart);

        self.matched_items.items.clear();

        for entry in &self.entries {
            let mut indices = Vec::new();
            let score = self.pattern.indices(
                Utf32String::from(entry.to_string()).slice(..),
                &mut self.matcher,
                &mut indices,
            );

            if score.is_some() {
                indices.sort_unstable();
                indices.dedup();

                self.matched_items
                    .items
                    .push(MatchedItem::new(entry.clone(), score, &indices));
            }
        }

        self.match_count = self.matched_items.items.len();
        self.matched_items
            .items
            .sort_by(|a, b| b.score.cmp(&a.score));
        self.matched_items.select_first();
    }

    pub fn non_interactive<'a>(paths: &'a [String], pattern: &str) -> Vec<&'a String> {
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let pattern = Pattern::parse(pattern, CaseMatching::Ignore, Normalization::Smart);
        let matched_paths = pattern.match_list(paths, &mut matcher);

        matched_paths.into_iter().map(|(path, _)| path).collect()
    }

    pub fn run_inline(&mut self, profile: &Profile) -> Result<Option<&str>> {
        self.fill_paths(profile);
        self.update_matches();

        if self.matched_items.items.len() > 1 {
            let mut terminal = ui::init_inline(25);

            loop {
                terminal.draw(|f| ui::draw_fuzzy_finder(f, self, f.area()))?;

                if let Event::Key(key) = crossterm::event::read()? {
                    match key.code {
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Esc => {
                            self.matched_items.state.select(None);
                            break;
                        }
                        _ => handle_key_fuzzy_mode(key, self),
                    }
                }
            }

            terminal.clear()?;
            ui::restore();
        }

        let selected_item = self.matched_items.get_selected();
        Ok(selected_item.map(|item| item.text.as_str()))
    }

    pub fn is_active(&self) -> bool {
        !self.entries.is_empty()
    }
}
