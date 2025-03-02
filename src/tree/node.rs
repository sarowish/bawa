use std::ops::{Deref, DerefMut};

use super::id::NodeId;

pub struct Node<T> {
    pub(super) parent: Option<NodeId>,
    pub(super) previous_sibling: Option<NodeId>,
    pub(super) next_sibling: Option<NodeId>,
    pub(super) first_child: Option<NodeId>,
    pub(super) last_child: Option<NodeId>,
    pub expanded: Option<bool>,
    pub value: T,
}

impl<T> Node<T> {
    /// Creates a node with the given value, with its relations set to `None`.
    pub fn new(value: T) -> Self {
        Self {
            parent: None,
            previous_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            expanded: None,
            value,
        }
    }

    /// Returns the `id` of the parent node or `None` if the node is a root of the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// tree.append(r, a);
    ///
    /// assert_eq!(tree[a].parent(), Some(r));
    /// assert!(tree[r].parent().is_none());
    /// ```
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Returns the `id` of the previous sibling or `None` if the node is the first child of its
    /// parent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// tree.append(r, a);
    /// tree.append(r, b);
    ///
    /// assert_eq!(tree[b].previous_sibling(), Some(a));
    /// assert!(tree[a].previous_sibling().is_none());
    /// ```
    pub fn previous_sibling(&self) -> Option<NodeId> {
        self.previous_sibling
    }

    /// Returns the `id` of the next sibling or `None` if the node is the last child of its
    /// parent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// tree.append(r, a);
    /// tree.append(r, b);
    ///
    /// assert_eq!(tree[a].next_sibling(), Some(b));
    /// assert!(tree[b].next_sibling().is_none());
    /// ```
    pub fn next_sibling(&self) -> Option<NodeId> {
        self.next_sibling
    }

    /// Returns the `id` of the first child or `None` if the node has no children.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    ///
    /// assert!(tree[r].first_child().is_none());
    ///
    /// tree.append(r, a);
    /// tree.append(r, b);
    ///
    /// assert_eq!(tree[r].first_child(), Some(a));
    /// ```
    pub fn first_child(&self) -> Option<NodeId> {
        self.first_child
    }

    /// Returns the `id` of the last child or `None` if the node has no children.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    ///
    /// assert!(tree[r].last_child().is_none());
    ///
    /// tree.append(r, a);
    /// tree.append(r, b);
    ///
    /// assert_eq!(tree[r].last_child(), Some(b));
    /// ```
    pub fn last_child(&self) -> Option<NodeId> {
        self.last_child
    }

    pub fn has_children(&self) -> bool {
        self.first_child.is_some()
    }

    pub fn non_root_parent(&self) -> Option<NodeId> {
        self.parent.filter(|id| *id != NodeId::root())
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded.is_some_and(|b| b)
    }

    pub fn is_collapsed(&self) -> bool {
        self.expanded.is_some_and(|b| !b)
    }

    pub fn toggle_fold(&mut self) {
        self.expanded = self.expanded.map(|t| !t);
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
