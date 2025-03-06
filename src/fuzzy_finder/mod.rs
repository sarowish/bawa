use crate::{
    app::StatefulList,
    input::{handle_key_fuzzy_mode, Input},
    ui,
};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use item::Matched;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32String,
};
use picker::Picker;

mod item;
pub mod picker;

pub struct FuzzyFinder {
    matcher: Matcher,
    pub input: Input,
    pattern: Pattern,
    pub picker: Option<Box<dyn Picker>>,
    pub matched: StatefulList<Matched>,
    pub total_count: usize,
    pub match_count: usize,
}

impl Default for FuzzyFinder {
    fn default() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            input: Input::with_prompt("> "),
            pattern: Pattern::default(),
            picker: None,
            matched: StatefulList::with_items(Vec::new()),
            total_count: 0,
            match_count: 0,
        }
    }
}

impl FuzzyFinder {
    fn items(&self) -> Option<Vec<Utf32String>> {
        self.picker.as_ref().map(|picker| picker.items())
    }

    pub fn selected_idx(&self) -> Option<usize> {
        self.matched.get_selected().map(|item| item.idx)
    }

    pub fn reset(&mut self) {
        self.input.set_text("");
        self.picker.take();
    }

    pub fn update_matches(&mut self) {
        let Some(items) = self.items() else {
            return;
        };

        self.pattern
            .reparse(&self.input.text, CaseMatching::Smart, Normalization::Smart);

        self.matched.items.clear();

        for (idx, path) in items.iter().enumerate() {
            let mut indices = Vec::new();
            let score = self
                .pattern
                .indices(path.slice(..), &mut self.matcher, &mut indices);

            if score.is_some() {
                indices.sort_unstable();
                indices.dedup();

                self.matched
                    .items
                    .push(Matched::new(path.to_string(), idx, score, &indices));
            }
        }

        self.match_count = self.matched.items.len();
        self.matched.items.sort_by(|a, b| b.score.cmp(&a.score));
        self.matched.select_first();
    }

    pub fn non_interactive<'a>(paths: &'a [String], pattern: &str) -> Vec<&'a String> {
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let pattern = Pattern::parse(pattern, CaseMatching::Ignore, Normalization::Smart);
        let matched_paths = pattern.match_list(paths, &mut matcher);

        matched_paths.into_iter().map(|(path, _)| path).collect()
    }

    pub fn run_inline(&mut self) -> Result<Option<&str>> {
        self.update_matches();

        if self.matched.items.len() > 1 {
            let mut terminal = ui::init_inline(25);

            loop {
                terminal.draw(|f| ui::draw_fuzzy_finder(f, self, f.area()))?;

                if let Event::Key(key) = crossterm::event::read()? {
                    match key.code {
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Esc => {
                            self.matched.state.select(None);
                            break;
                        }
                        _ => handle_key_fuzzy_mode(key, self),
                    }
                }
            }

            terminal.clear()?;
            ui::restore();
        }

        let selected_item = self.matched.get_selected();
        Ok(selected_item.map(|item| item.text.as_str()))
    }

    pub fn is_active(&self) -> bool {
        self.picker.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{FuzzyFinder, Matched};

    #[test]
    fn fuzzy_unicode() {
        let item = Matched::new("türkçe".to_owned(), 0, None, &[1, 2]);

        assert_eq!(
            item.highlight_slices(),
            vec![("t", false), ("ür", true), ("kçe", false)]
        );
    }

    #[test]
    fn fuzzy_unicode2() {
        let item = Matched::new("türkçe".to_owned(), 0, None, &[0]);

        assert_eq!(item.highlight_slices(), vec![("t", true), ("ürkçe", false)]);
    }

    #[test]
    fn fuzzy_postfix() {
        let item = Matched::new("some text".to_owned(), 0, None, &[6, 7, 8]);

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
