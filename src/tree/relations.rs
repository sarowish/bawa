use super::{id::NodeId, Tree};

impl<T> Tree<T> {
    pub fn on_insert(
        &mut self,
        new: NodeId,
        parent: Option<NodeId>,
        prev_sibling: Option<NodeId>,
        next_sibling: Option<NodeId>,
    ) {
        self[new].parent = parent;
        self.update_neighbours(parent, prev_sibling, Some(new));
        self.update_neighbours(parent, Some(new), next_sibling);
    }

    pub fn update_neighbours(
        &mut self,
        parent: Option<NodeId>,
        prev: Option<NodeId>,
        next: Option<NodeId>,
    ) {
        if let Some(prev_node) = prev.map(|id| &mut self[id]) {
            prev_node.next_sibling = next;
        } else if let Some(id) = parent {
            self[id].first_child = next;
        }

        if let Some(next_node) = next.map(|id| &mut self[id]) {
            next_node.previous_sibling = prev;
        } else if let Some(id) = parent {
            self[id].last_child = prev;
        }
    }
}
