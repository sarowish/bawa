use super::{state::TreeState, tree::Tree};
use crate::tree::NodeId;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::BlockExt,
    text::Span,
    widgets::{StatefulWidget, Widget},
};

impl StatefulWidget for Tree<'_> {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);
        if let Some(block) = &self.block {
            block.render(area, buf);
        }
        let tree_area = self.block.inner_if_some(area);

        if tree_area.is_empty() || self.items.is_empty() {
            return;
        }

        let tree_height = tree_area.height as usize;

        let (first_visible_idx, last_visible_idx) =
            self.get_items_bounds(state.selected, state.offset, tree_height);

        state.offset = first_visible_idx;

        for (i, mut item) in self
            .items
            .into_iter()
            .skip(state.offset)
            .take(last_visible_idx - first_visible_idx + 1)
            .enumerate()
        {
            let (x, y) = (tree_area.left(), tree_area.top() + i as u16);

            let row_area = Rect {
                x,
                y,
                width: tree_area.width,
                height: 1,
            };

            let item_style = self.style.patch(item.style);
            buf.set_style(row_area, item_style);

            let is_selected = state.selected.filter(|id| *id == item.id).is_some();

            if state.marked.contains(&item.id)
                && let Some(span) = item
                    .content
                    .iter_mut()
                    .last()
                    .and_then(|line| line.spans.last_mut())
            {
                span.style = self.marked_style;
            }

            if state.active.filter(|id| *id == item.id).is_some() {
                item.content
                    .push_span(Span::styled(" (*)", self.active_style));
            }

            item.content.render(row_area, buf);

            if is_selected {
                buf.set_style(row_area, self.highlight_style);
            }
        }
    }
}

impl Tree<'_> {
    fn get_items_bounds(
        &self,
        selected: Option<NodeId>,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(self.items.len().saturating_sub(1));

        let mut first_visible_idx = offset;
        let mut last_visible_idx = offset;

        let height_from_offset = self.items.iter().skip(offset).count().min(max_height);
        last_visible_idx += height_from_offset - 1;

        let index_to_display = self
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| Some(item.id) == selected)
            .map_or(offset, |(idx, _)| idx);

        if index_to_display > last_visible_idx {
            first_visible_idx += index_to_display - last_visible_idx;
            last_visible_idx = index_to_display;
        } else if index_to_display < first_visible_idx {
            first_visible_idx = index_to_display;
            last_visible_idx =
                first_visible_idx + (self.items.len() - first_visible_idx).min(max_height) - 1;
        }

        (first_visible_idx, last_visible_idx)
    }
}

#[cfg(test)]
mod tests {
    use crate::tree::{
        NodeId,
        widget::{item::TreeItem, state::TreeState, tree::Tree},
    };

    #[test]
    fn selected_within_view() {
        let items = vec![
            TreeItem {
                content: "a".into(),
                style: Default::default(),
                id: NodeId::new(0),
            },
            TreeItem {
                content: "b".into(),
                style: Default::default(),
                id: NodeId::new(1),
            },
            TreeItem {
                content: "c".into(),
                style: Default::default(),
                id: NodeId::new(2),
            },
            TreeItem {
                content: "d".into(),
                style: Default::default(),
                id: NodeId::new(3),
            },
        ];
        let widget = Tree::new(items);
        let mut state = TreeState::default();
        state.select_unchecked(Some(NodeId::new(2)));

        let (first, last) = widget.get_items_bounds(state.selected, 0, 3);

        assert_eq!(first, 0);
        assert_eq!(last, 2);

        let (first, last) = widget.get_items_bounds(state.selected, 0, 2);

        assert_eq!(first, 1);
        assert_eq!(last, 2);

        let (first, last) = widget.get_items_bounds(state.selected, 2, 10);

        assert_eq!(first, 2);
        assert_eq!(last, 3);

        let (first, last) = widget.get_items_bounds(state.selected, 0, 10);

        assert_eq!(first, 0);
        assert_eq!(last, 3);
    }

    #[test]
    fn selected_lower_than_offset() {
        let items = vec![
            TreeItem {
                content: "a".into(),
                style: Default::default(),
                id: NodeId::new(0),
            },
            TreeItem {
                content: "b".into(),
                style: Default::default(),
                id: NodeId::new(1),
            },
            TreeItem {
                content: "c".into(),
                style: Default::default(),
                id: NodeId::new(2),
            },
            TreeItem {
                content: "d".into(),
                style: Default::default(),
                id: NodeId::new(3),
            },
        ];
        let widget = Tree::new(items);
        let mut state = TreeState::default();
        state.select_unchecked(Some(NodeId::new(1)));

        let (first, last) = widget.get_items_bounds(state.selected, 3, 4);

        assert_eq!(first, 1);
        assert_eq!(last, 3);
    }

    #[test]
    fn selected_higher_than_max_height() {
        let items = vec![
            TreeItem {
                content: "a".into(),
                style: Default::default(),
                id: NodeId::new(0),
            },
            TreeItem {
                content: "b".into(),
                style: Default::default(),
                id: NodeId::new(1),
            },
            TreeItem {
                content: "c".into(),
                style: Default::default(),
                id: NodeId::new(2),
            },
            TreeItem {
                content: "d".into(),
                style: Default::default(),
                id: NodeId::new(3),
            },
        ];
        let widget = Tree::new(items);
        let mut state = TreeState::default();
        state.select_unchecked(Some(NodeId::new(2)));

        let (first, last) = widget.get_items_bounds(state.selected, 0, 2);

        assert_eq!(first, 1);
        assert_eq!(last, 2);
    }
}
