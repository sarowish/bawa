use crate::tree::{NodeId, Tree};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};
use std::fmt::Display;

pub struct TreeItem<'a> {
    pub content: Text<'a>,
    pub style: Style,
    pub id: NodeId,
}

impl TreeItem<'_> {
    pub fn new<T>(id: NodeId, depth: usize, last_item: bool, tree: &Tree<T>) -> Self
    where
        T: Display,
    {
        let indent_guides = if last_item {
            format!("{}  └ ", "  │ ".repeat(depth - 1))
        } else {
            "  │ ".repeat(depth)
        };
        let folder = match tree[id].expanded {
            Some(true) => "  ",
            Some(false) => "  ",
            None => " ",
        };

        let line = Line::from(vec![
            Span::styled(indent_guides, Color::DarkGray),
            Span::raw(folder),
            Span::raw(tree[id].to_string()),
        ]);

        Self {
            content: line.into(),
            style: Style::default(),
            id,
        }
    }
}
