use super::{Node, NodeId, Tree};

macro_rules! gen_iter {
    ($name:ident, next = $next:expr) => {
        pub struct $name<'a, T> {
            tree: &'a Tree<T>,
            node: Option<NodeId>,
        }

        impl<'a, T> $name<'a, T> {
            pub fn new(node: NodeId, tree: &'a Tree<T>) -> Self {
                Self {
                    node: Some(node),
                    tree,
                }
            }
        }

        impl<T> Iterator for $name<'_, T> {
            type Item = NodeId;

            fn next(&mut self) -> Option<Self::Item> {
                let next: fn(&Node<T>) -> Option<NodeId> = $next;
                let node = self.node.take();
                self.node = node.and_then(|id| next(&self.tree[id]));
                node
            }
        }
    };
    ($name:ident, new = $new:expr, next = $next:expr, prev = $prev:expr) => {
        pub struct $name<'a, T> {
            tree: &'a Tree<T>,
            head: Option<NodeId>,
            tail: Option<NodeId>,
        }

        impl<'a, T> $name<'a, T> {
            pub fn new(id: NodeId, tree: &'a Tree<T>) -> Self {
                let new: fn(&Tree<T>, NodeId) -> (Option<NodeId>, Option<NodeId>) = $new;
                let (head, tail) = new(tree, id);
                Self { head, tail, tree }
            }
        }

        impl<T> Iterator for $name<'_, T> {
            type Item = NodeId;

            fn next(&mut self) -> Option<Self::Item> {
                let next: fn(&Node<T>) -> Option<NodeId> = $next;
                let node = self.head.take();
                self.head = node.and_then(|id| next(&self.tree[id]));
                node
            }
        }

        impl<T> DoubleEndedIterator for $name<'_, T> {
            fn next_back(&mut self) -> Option<Self::Item> {
                let prev: fn(&Node<T>) -> Option<NodeId> = $prev;
                let node = self.tail.take();
                self.tail = node.and_then(|id| prev(&self.tree[id]));
                node
            }
        }
    };
}

gen_iter!(
    Children,
    new = |tree, id| (tree[id].first_child, tree[id].last_child),
    next = |node| node.next_sibling,
    prev = |node| node.previous_sibling
);

gen_iter!(Ancestors, next = |node| node.parent);

gen_iter!(
    Predecessors,
    next = |node| node.previous_sibling.or(node.parent)
);

gen_iter!(
    FollowingSiblings,
    new = |tree, id| {
        (
            tree[id].next_sibling,
            tree[id]
                .parent
                .and_then(|parent_id| tree[parent_id].last_child),
        )
    },
    next = |node| node.next_sibling,
    prev = |node| node.previous_sibling
);

gen_iter!(
    PrecedingSiblings,
    new = |tree, id| {
        (
            tree[id].previous_sibling,
            tree[id]
                .parent
                .and_then(|parent_id| tree[parent_id].first_child),
        )
    },
    next = |node| node.previous_sibling,
    prev = |node| node.next_sibling
);

pub struct Descendants<'a, T>(Traverse<'a, T>);

impl<'a, T> Descendants<'a, T> {
    pub fn new(ancestor: NodeId, tree: &'a Tree<T>) -> Self {
        Self(Traverse::new(ancestor, tree))
    }
}

impl<T> Iterator for Descendants<'_, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        next_start(&mut self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edge {
    Start(NodeId),
    End(NodeId),
}

impl Edge {
    /// Returns the next edge.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// # use bawa::tree::traverse::Edge;
    /// use std::iter::successors;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_d_e = tree.add_value("e");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(a_d, a_d_e);
    /// tree.append(r, b);
    ///
    /// let mut edges = successors(Some(Edge::Start(r)), |edge| edge.next(&tree));
    ///
    /// assert_eq!(edges.next(), Some(Edge::Start(r)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a_c)));
    /// assert_eq!(edges.next(), Some(Edge::End(a_c)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a_d)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a_d_e)));
    /// assert_eq!(edges.next(), Some(Edge::End(a_d_e)));
    /// assert_eq!(edges.next(), Some(Edge::End(a_d)));
    /// assert_eq!(edges.next(), Some(Edge::End(a)));
    /// assert_eq!(edges.next(), Some(Edge::Start(b)));
    /// assert_eq!(edges.next(), Some(Edge::End(b)));
    /// assert_eq!(edges.next(), Some(Edge::End(r)));
    ///
    /// assert_eq!(edges.next(), None);
    /// ```
    pub fn next<T>(self, tree: &Tree<T>) -> Option<Self> {
        self.next_helper(tree, false)
    }

    /// Returns the next visible edge.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// # use bawa::tree::traverse::Edge;
    /// use std::iter::successors;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_d_e = tree.add_value("e");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(a_d, a_d_e);
    /// tree.append(r, b);
    ///
    /// tree[r].expanded = Some(true);
    ///
    /// let mut edges = successors(Some(Edge::Start(r)), |edge| edge.next_visible(&tree));
    ///
    /// assert_eq!(edges.next(), Some(Edge::Start(r)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a)));
    /// assert_eq!(edges.next(), Some(Edge::End(a)));
    /// assert_eq!(edges.next(), Some(Edge::Start(b)));
    /// assert_eq!(edges.next(), Some(Edge::End(b)));
    /// assert_eq!(edges.next(), Some(Edge::End(r)));
    ///
    /// assert_eq!(edges.next(), None);
    /// ```
    pub fn next_visible<T>(self, tree: &Tree<T>) -> Option<Self> {
        self.next_helper(tree, true)
    }

    fn next_helper<T>(self, tree: &Tree<T>, skip_invisible: bool) -> Option<Self> {
        match self {
            Self::Start(id) => Some(&tree[id])
                .filter(|node| !skip_invisible || node.is_expanded())
                .and_then(|node| node.first_child)
                .map(Self::Start)
                .or(Some(Self::End(id))),
            Self::End(id) => tree[id]
                .next_sibling
                .map(Self::Start)
                .or(tree[id].parent.map(Self::End)),
        }
    }

    /// Returns the previous edge.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// # use bawa::tree::traverse::Edge;
    /// use std::iter::successors;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_d_e = tree.add_value("e");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(a_d, a_d_e);
    /// tree.append(r, b);
    ///
    /// let mut edges = successors(Some(Edge::End(r)), |edge| edge.prev(&tree));
    ///
    /// assert_eq!(edges.next(), Some(Edge::End(r)));
    /// assert_eq!(edges.next(), Some(Edge::End(b)));
    /// assert_eq!(edges.next(), Some(Edge::Start(b)));
    /// assert_eq!(edges.next(), Some(Edge::End(a)));
    /// assert_eq!(edges.next(), Some(Edge::End(a_d)));
    /// assert_eq!(edges.next(), Some(Edge::End(a_d_e)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a_d_e)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a_d)));
    /// assert_eq!(edges.next(), Some(Edge::End(a_c)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a_c)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a)));
    /// assert_eq!(edges.next(), Some(Edge::Start(r)));
    ///
    /// assert_eq!(edges.next(), None);
    /// ```
    pub fn prev<T>(self, tree: &Tree<T>) -> Option<Self> {
        self.prev_helper(tree, false)
    }

    /// Returns the previous visible edge.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// # use bawa::tree::traverse::Edge;
    /// use std::iter::successors;
    ///
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_d_e = tree.add_value("e");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(a_d, a_d_e);
    /// tree.append(r, b);
    ///
    /// tree[r].expanded = Some(true);
    ///
    /// let mut edges = successors(Some(Edge::End(r)), |edge| edge.prev_visible(&tree));
    ///
    /// assert_eq!(edges.next(), Some(Edge::End(r)));
    /// assert_eq!(edges.next(), Some(Edge::End(b)));
    /// assert_eq!(edges.next(), Some(Edge::Start(b)));
    /// assert_eq!(edges.next(), Some(Edge::End(a)));
    /// assert_eq!(edges.next(), Some(Edge::Start(a)));
    /// assert_eq!(edges.next(), Some(Edge::Start(r)));
    ///
    /// assert_eq!(edges.next(), None);
    /// ```
    pub fn prev_visible<T>(self, tree: &Tree<T>) -> Option<Self> {
        self.prev_helper(tree, true)
    }

    fn prev_helper<T>(self, tree: &Tree<T>, skip_invisible: bool) -> Option<Self> {
        match self {
            Self::Start(id) => tree[id]
                .previous_sibling
                .map(Self::End)
                .or(tree[id].parent.map(Self::Start)),
            Self::End(id) => Some(&tree[id])
                .filter(|node| !skip_invisible || node.is_expanded())
                .and_then(|node| node.last_child)
                .map(Self::End)
                .or(Some(Self::Start(id))),
        }
    }
}

pub struct Traverse<'a, T> {
    tree: &'a Tree<T>,
    head: Option<Edge>,
    tail: Option<Edge>,
    visible: bool,
}

impl<'a, T> Traverse<'a, T> {
    pub fn new(root: NodeId, tree: &'a Tree<T>) -> Self {
        Self {
            tree,
            head: Some(Edge::Start(root)),
            tail: Some(Edge::End(root)),
            visible: false,
        }
    }

    pub fn visible(mut self) -> Self {
        self.visible = true;
        self
    }

    pub fn from(mut self, id: NodeId) -> Self {
        self.head = Some(Edge::Start(id));
        self
    }

    pub fn to(mut self, id: NodeId) -> Self {
        self.tail = Some(Edge::Start(id));
        self
    }
}

impl<T> Iterator for Traverse<'_, T> {
    type Item = Edge;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.head.take();

        self.head = current
            .and_then(|edge| {
                if self.visible {
                    edge.next_visible(self.tree)
                } else {
                    edge.next(self.tree)
                }
            })
            .filter(|edge| Some(edge) != self.tail.as_ref());

        current
    }
}

impl<T> DoubleEndedIterator for Traverse<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.tail.take();

        self.tail = current
            .and_then(|edge| {
                if self.visible {
                    edge.prev_visible(self.tree)
                } else {
                    edge.prev(self.tree)
                }
            })
            .filter(|edge| Some(edge) != self.head.as_ref());

        current
    }
}

pub fn next_start(it: &mut impl Iterator<Item = Edge>) -> Option<NodeId> {
    it.find_map(|edge| match edge {
        Edge::Start(id) => Some(id),
        Edge::End(_) => None,
    })
}
