use crate::{
    app::App,
    config::OPTIONS,
    input::{Mode, SearchContext},
    tree::NodeId,
};
use anyhow::Result;
use nucleo_matcher::{
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
    Matcher, Utf32String,
};

pub enum Direction {
    Forward,
    Backward,
}

#[derive(Default)]
pub struct Search {
    matcher: Matcher,
    pub matches: Vec<usize>,
    pub pattern: String,
    pub start_idx: Option<usize>,
}

impl Search {
    /// Searches for `pattern` in the list and the given direction. Returns the index of the first
    /// match.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::search::{Search, Direction};
    /// # use nucleo_matcher::Utf32String;
    ///
    /// let mut search = Search::default();
    /// search.pattern = String::from("ap");
    ///
    /// let list = ["apple", "pineapple", "clementine"]
    ///     .iter()
    ///     .map(|s| Utf32String::from(*s))
    ///     .collect::<Vec<_>>();
    ///
    /// let idx = search.search(&list, Direction::Forward);
    /// assert_eq!(idx, Some(0));
    ///
    /// let idx = search.search(&list, Direction::Backward);
    /// assert_eq!(idx, Some(1));
    ///
    /// search.starting_index = Some(0);
    /// let idx = search.search(&list, Direction::Forward);
    /// assert_eq!(idx, Some(1));
    /// ```
    pub fn search(&mut self, list: &[Utf32String], direction: Direction) -> Option<usize> {
        let pattern = Pattern::new(
            &self.pattern,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Substring,
        );

        self.matches = list
            .iter()
            .enumerate()
            .filter(|(_, s)| pattern.score(s.slice(..), &mut self.matcher).is_some())
            .map(|(idx, _)| idx)
            .collect();

        match direction {
            Direction::Forward => self.next_match(),
            Direction::Backward => self.previous_match(),
        }
        .copied()
    }

    /// Returns the index of the first match after `start_idx` or the first match if it is `None`.
    pub fn next_match(&mut self) -> Option<&usize> {
        self.start_idx
            .and_then(|start| self.matches.iter().find(|idx| **idx > start))
            .or(self.matches.first())
    }

    /// Returns the index of the first match before `start_idx` or the last match if it is `None`.
    pub fn previous_match(&mut self) -> Option<&usize> {
        self.start_idx
            .and_then(|start| self.matches.iter().rev().find(|idx| **idx < start))
            .or(self.matches.last())
    }

    /// Returns true if the pattern isn't empty and there isn't a match.
    pub fn no_match(&self) -> bool {
        !self.pattern.is_empty() && self.matches.is_empty()
    }
}

impl App {
    pub fn search_new_pattern(&mut self) {
        self.take_input(Mode::Search(self.mode.search_context()));
        self.search.start_idx = self.get_search_start_position();
    }

    pub fn run_search(&mut self, direction: Direction) {
        if self.search.pattern.is_empty() {
            self.jump_to_match(self.search.start_idx);
            return;
        }

        let items: Vec<_> = match self.mode.search_context() {
            SearchContext::Normal => {
                let entries = self.profiles.get_entries().unwrap();
                entries
                    .visible(NodeId::root())
                    .map(|id| Utf32String::from(entries[id].to_string()))
                    .collect()
            }
            SearchContext::ProfileSelection => self
                .profiles
                .inner
                .items
                .iter()
                .map(|p| Utf32String::from(p.name()))
                .collect(),
        };

        if let Some(idx) = self.search.search(&items, direction) {
            self.message.clear();
            self.jump_to_match(Some(idx));
        } else {
            self.jump_to_match(self.search.start_idx);
            self.message
                .set_error_from_str(&format!("Pattern not found: {}", self.search.pattern));
        }
    }

    pub fn complete_search(&mut self) -> Result<()> {
        self.search.pattern = self.extract_input();

        if !OPTIONS.incremental_search {
            self.run_search(Direction::Forward);
        }

        if self.search.no_match() {
            self.message
                .set_error_from_str(&format!("Pattern not found: {}", self.search.pattern));
        }

        self.search.start_idx = None;

        Ok(())
    }

    pub fn abort_search(&mut self) {
        self.abort_input();
        let idx = self.search.start_idx.take();
        self.jump_to_match(idx);
        self.search.pattern.clear();
    }

    pub fn repeat_search(&mut self) {
        self.search.start_idx = self.get_search_start_position();
        self.run_search(Direction::Forward);
    }

    pub fn repeat_search_reverse(&mut self) {
        self.search.start_idx = self.get_search_start_position();
        self.run_search(Direction::Backward);
    }

    fn get_search_start_position(&mut self) -> Option<usize> {
        match self.mode.search_context() {
            SearchContext::Normal => self.tree_state.selected.and_then(|selected| {
                self.profiles
                    .get_entries()
                    .unwrap()
                    .visible(NodeId::root())
                    .position(|id| id == selected)
            }),
            SearchContext::ProfileSelection => self.profiles.inner.state.selected(),
        }
    }

    pub fn jump_to_match(&mut self, idx: Option<usize>) {
        if let Some(idx) = idx {
            match self.mode.search_context() {
                SearchContext::Normal => {
                    let mut visible = self.profiles.get_entries().unwrap().visible(NodeId::root());
                    self.tree_state.select_unchecked(visible.nth(idx));
                    self.auto_mark_save_file();
                }
                SearchContext::ProfileSelection => self.profiles.inner.state.select(Some(idx)),
            };
        }
    }
}
