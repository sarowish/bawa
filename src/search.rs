use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Matcher, Utf32String,
};

#[derive(Default)]
pub struct Search {
    matcher: Matcher,
    pub pattern: String,
}

impl Search {
    pub fn search(&mut self, list: &[Utf32String]) -> Vec<usize> {
        let pattern = Pattern::new(
            &self.pattern,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Substring,
        );

        list.iter()
            .enumerate()
            .filter(|(_, s)| pattern.score(s.slice(..), &mut self.matcher).is_some())
            .map(|(idx, _)| idx)
            .collect()
    }
}
