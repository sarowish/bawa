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
            pattern: pattern.to_lowercase(),
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

