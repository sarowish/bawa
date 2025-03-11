use ratatui::{layout::Rect, text::Line};

#[derive(Default)]
pub struct Scroller {
    offset: u16,
    length: u16,
}

impl Scroller {
    pub fn scroll_up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.offset = self.offset.saturating_add(1).min(self.length);
    }

    pub fn scroll_top(&mut self) {
        self.offset = 0;
    }

    pub fn scroll_bottom(&mut self) {
        self.offset = self.length;
    }

    pub fn offset(&mut self, area: Rect, content: &[Line]) -> u16 {
        let width = area.width.max(1);

        self.length = content
            .iter()
            .map(|entry| 1 + entry.width().saturating_sub(1) as u16 / width)
            .sum::<u16>()
            .saturating_sub(area.height);

        if self.length < self.offset {
            self.offset = self.length;
        }

        self.offset
    }

    pub fn length(&self) -> usize {
        self.length.into()
    }
}
