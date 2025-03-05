#[derive(Debug, PartialEq, Eq)]
pub struct Matched {
    pub text: String,
    pub idx: usize,
    pub score: Option<u32>,
    pub indices: Vec<u32>,
}

impl Matched {
    pub fn new(text: String, idx: usize, score: Option<u32>, indices: &[u32]) -> Self {
        Self {
            text,
            idx,
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
        let upper_boundary_map = upper_char_boundaries(text);

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

pub fn upper_char_boundaries(text: &str) -> Vec<usize> {
    (1..=text.len())
        .filter(|idx| text.is_char_boundary(*idx))
        .collect()
}
