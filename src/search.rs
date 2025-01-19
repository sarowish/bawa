use crate::{
    app::StatefulList,
    input::{handle_key_fuzzy_mode, Input},
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

#[derive(Debug, PartialEq, Eq)]
pub struct MatchedItem {
    pub text: String,
    score: Option<u32>,
    indices: Vec<u32>,
}

impl MatchedItem {
    pub fn new(text: String, score: Option<u32>, indices: &[u32]) -> Self {
        Self {
            text,
            score,
            indices: Vec::from(indices),
        }
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

pub struct FuzzyFinder {
    matcher: Matcher,
    pub input: Input,
    pattern: Pattern,
    paths: Vec<Utf32String>,
    pub matched_items: StatefulList<MatchedItem>,
}

impl FuzzyFinder {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            input: Input::new(),
            pattern: Pattern::default(),
            paths: Vec::new(),
            matched_items: StatefulList::with_items(Vec::new()),
        }
    }

    pub fn fill_paths(&mut self, paths: &[String]) {
        self.paths = paths
            .iter()
            .map(|path| Utf32String::from(path.clone()))
            .collect();
    }

    pub fn reset(&mut self) {
        self.input.set_text("");
        self.paths.drain(..);
    }

    pub fn update_matches(&mut self) {
        self.pattern
            .reparse(&self.input.text, CaseMatching::Smart, Normalization::Smart);

        self.matched_items.items.clear();

        for path in &self.paths {
            let mut indices = Vec::new();
            let score = self
                .pattern
                .indices(path.slice(..), &mut self.matcher, &mut indices);

            if score.is_some() {
                indices.sort_unstable();
                indices.dedup();

                self.matched_items
                    .items
                    .push(MatchedItem::new(path.to_string(), score, &indices));
            }
        }

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

    pub fn run_inline(&mut self, paths: &[String]) -> Result<Option<&str>> {
        self.fill_paths(paths);
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
        !self.paths.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::FuzzyFinder;
    use crate::search::MatchedItem;

    #[test]
    fn fuzzy_unicode() {
        let item = MatchedItem::new("türkçe".to_string(), None, &[1, 2]);

        assert_eq!(
            item.highlight_slices(),
            vec![("t", false), ("ür", true), ("kçe", false)]
        );
    }

    #[test]
    fn fuzzy_unicode2() {
        let item = MatchedItem::new("türkçe".to_string(), None, &[0]);

        assert_eq!(item.highlight_slices(), vec![("t", true), ("ürkçe", false)]);
    }

    #[test]
    fn fuzzy_postfix() {
        let item = MatchedItem::new("some text".to_string(), None, &[6, 7, 8]);

        assert_eq!(
            item.highlight_slices(),
            vec![("some t", false), ("ext", true)]
        );
    }

    #[test]
    fn oneshot() {
        let strings = &[
            String::from("Altus Plateau/Godskin Apostle.sl2"),
            String::from("Mt. Gelmir/Godskin Noble.sl2"),
            String::from("Altus Plateau/Leyndell/Goldfrey.sl2"),
        ];

        let matches = FuzzyFinder::non_interactive(strings, "godnob");

        assert_eq!(matches.len(), 1);
    }
}
