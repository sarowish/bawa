use crate::{app::StatefulList, input::Input, utils};
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
    pub input: Option<Input>,
    pattern: Pattern,
    paths: Vec<Utf32String>,
    pub matched_items: StatefulList<MatchedItem>,
}

impl FuzzyFinder {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            input: None,
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

    pub fn update_matches(&mut self) {
        self.pattern.reparse(
            &self.input.as_ref().unwrap().text,
            CaseMatching::Smart,
            Normalization::Smart,
        );

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

    pub fn is_active(&self) -> bool {
        self.input.is_some()
    }
}

#[cfg(test)]
mod tests {
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
}
