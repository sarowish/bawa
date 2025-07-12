use super::item::TreeItem;
use crate::tree::{
    NodeId,
    traverse::{Edge, Traverse},
};
use ratatui::{style::Style, widgets::Block};
use std::fmt::Display;

#[derive(Default)]
pub struct Tree<'a> {
    pub block: Option<Block<'a>>,
    pub items: Vec<TreeItem<'a>>,
    pub style: Style,
    pub highlight_style: Style,
    pub marked_style: Style,
    pub active_style: Style,
}

impl<'a> Tree<'a> {
    pub fn new<T>(items: T) -> Self
    where
        T: IntoIterator<Item: Into<TreeItem<'a>>>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn marked_style(mut self, style: Style) -> Self {
        self.marked_style = style;
        self
    }

    pub fn active_style(mut self, style: Style) -> Self {
        self.active_style = style;
        self
    }
}

impl<T> From<&crate::tree::Tree<T>> for Tree<'_>
where
    T: Display,
{
    fn from(tree: &crate::tree::Tree<T>) -> Self {
        let mut items = Vec::new();
        let mut depth = 0;

        for edge in Traverse::new(NodeId::root(), tree).visible().skip(1) {
            match edge {
                Edge::Start(id) => {
                    let node = &tree[id];
                    let last_item =
                        depth != 0 && node.next_sibling.is_none() && !node.has_children();
                    items.push(TreeItem::new(id, depth, last_item, tree));
                    depth += 1;
                }
                Edge::End(_) => depth -= 1,
            }
        }

        Tree::new(items)
    }
}
