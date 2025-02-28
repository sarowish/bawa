use crate::tree::{
    traverse::{next_start, Traverse},
    NodeId, Tree,
};
use std::collections::HashSet;

#[derive(Default)]
pub struct TreeState {
    pub selected: Option<NodeId>,
    pub offset: usize,
    pub marked: HashSet<NodeId>,
    pub active: Option<NodeId>,
}

impl TreeState {
    /// Sets the node with the given `id` as the selected item.
    ///
    /// Sets to `None` if no node is selected. Expands ancestor nodes if they are not expanded.
    ///
    /// # Examples
    /// ```
    /// use bawa::tree::Tree;
    /// use bawa::tree::NodeId;
    /// use bawa::tree::TreeState;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let a_b = tree.add_value("b");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_b);
    ///
    /// let mut state = TreeState::default();
    ///
    /// assert!(!tree[a].is_expanded());
    ///
    /// state.select(Some(a_b), &mut tree);
    ///
    /// assert!(tree[a].is_expanded());
    /// ```
    pub fn select<T>(&mut self, id: Option<NodeId>, tree: &mut Tree<T>) {
        self.selected = id;

        if let Some(id) = id {
            for id in tree.ancestors(id).collect::<Vec<NodeId>>() {
                tree[id].expanded = Some(true)
            }
        }
    }

    /// Sets the node with the given `id` as the selected item.
    ///
    /// Sets to `None` if no node is selected. Doesn't ensure that the ancestor nodes are expanded.
    ///
    /// # Examples
    /// ```
    /// use bawa::tree::TreeState;
    /// use bawa::tree::NodeId;
    ///
    /// let mut state = TreeState::default();
    /// state.select_unchecked(Some(NodeId::new(1)));
    /// ```
    pub fn select_unchecked(&mut self, id: Option<NodeId>) {
        if matches!(id, Some(id) if id != NodeId::new(0)) {
            self.selected = id;
        }
    }

    pub fn mark(&mut self, id: NodeId) -> bool {
        self.marked.insert(id)
    }

    pub fn unmark(&mut self, id: NodeId) -> bool {
        self.marked.remove(&id)
    }

    /// Selects the next visible item or the first one if no item is selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use bawa::tree::Tree;
    /// use bawa::tree::TreeState;
    ///
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_c_d = tree.add_value("d");
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a_c, a_c_d);
    /// tree.append(r, b);
    ///
    /// let mut state = TreeState::default();
    /// tree[a].expanded = Some(true);
    ///
    /// state.select_next(&tree);
    /// assert_eq!(state.selected, Some(a));
    /// state.select_next(&tree);
    /// assert_eq!(state.selected, Some(a_c));
    /// state.select_next(&tree);
    /// assert_eq!(state.selected, Some(b));
    /// state.select_next(&tree);
    /// assert_eq!(state.selected, Some(a));
    /// tree[a].toggle_fold();
    /// state.select_next(&tree);
    /// assert_eq!(state.selected, Some(b));
    /// ```
    pub fn select_next<T>(&mut self, tree: &Tree<T>) {
        if let Some(id) = self.selected {
            self.selected = next_start(
                &mut Traverse::new(NodeId::new(0), tree)
                    .visible()
                    .from(id)
                    .skip(1),
            );
        }

        if self.selected.is_none() {
            self.select_first(tree);
        }
    }

    /// Selects the previous visible item or the last one if no item is selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use bawa::tree::Tree;
    /// use bawa::tree::TreeState;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_c_d = tree.add_value("d");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a_c, a_c_d);
    /// tree.append(r, b);
    ///
    /// let mut state = TreeState::default();
    /// tree[a].expanded = Some(true);
    ///
    /// state.select_prev(&tree);
    /// assert_eq!(state.selected, Some(b));
    /// state.select_prev(&tree);
    /// assert_eq!(state.selected, Some(a_c));
    /// state.select_prev(&tree);
    /// assert_eq!(state.selected, Some(a));
    /// state.select_prev(&tree);
    /// assert_eq!(state.selected, Some(b));
    /// tree[a].toggle_fold();
    /// state.select_prev(&tree);
    /// assert_eq!(state.selected, Some(a));
    /// ```
    pub fn select_prev<T>(&mut self, tree: &Tree<T>) {
        if let Some(id) = self.selected {
            self.selected = next_start(
                &mut Traverse::new(NodeId::new(0), tree)
                    .visible()
                    .to(id)
                    .rev()
                    .skip(1),
            );
        }

        if self.selected.is_none() {
            self.select_last(tree);
        }
    }

    /// Selects the first item of the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use bawa::tree::Tree;
    /// use bawa::tree::TreeState;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    ///
    /// tree.append(r, a);
    /// tree.append(r, b);
    ///
    /// let mut state = TreeState::default();
    /// state.select_first(&tree);
    ///
    /// assert_eq!(state.selected, Some(a));
    /// ```
    pub fn select_first<T>(&mut self, tree: &Tree<T>) {
        self.selected = tree.root().and_then(|node| node.first_child);
    }

    /// Selects the last visible item of the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use bawa::tree::Tree;
    /// use bawa::tree::TreeState;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let b_c =  tree.add_value("c");
    ///
    /// tree.append(r, a);
    /// tree.append(r, b);
    /// tree.append(b, b_c);
    ///
    /// let mut state = TreeState::default();
    /// tree[b].expanded = Some(true);
    /// state.select_last(&tree);
    ///
    /// assert_eq!(state.selected, Some(b_c));
    /// ```
    pub fn select_last<T>(&mut self, tree: &Tree<T>) {
        self.selected = std::iter::successors(tree.root().and_then(|node| node.last_child), |id| {
            tree.get(*id)
                .filter(|node| node.is_expanded())
                .and_then(|node| node.last_child)
        })
        .last();
    }
}
