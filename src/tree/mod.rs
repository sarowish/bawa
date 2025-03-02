pub use id::NodeId;
pub use node::Node;
use std::ops::{Index, IndexMut};
use traverse::{
    Ancestors, Children, Descendants, FollowingSiblings, PrecedingSiblings, Predecessors,
};
pub use widget::TreeState;

mod id;
mod node;
mod relations;
pub mod traverse;
pub mod widget;

macro_rules! skip_first {
    ($s:expr) => {{
        let mut s = $s;
        s.next();
        s
    }};
}

pub struct Tree<T> {
    nodes: Vec<Node<T>>,
}

impl<T> Tree<T> {
    /// Returns a reference to the node with the given id or `None` if there is no such node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::{Tree, NodeId};
    /// let mut tree = Tree::default();
    /// let id = tree.add_value(2);
    ///
    /// assert!(tree.get(id).is_some());
    /// assert!(tree.get(NodeId::new(10)).is_none());
    /// ```
    pub fn get(&self, id: NodeId) -> Option<&Node<T>> {
        self.nodes.get(id.index0())
    }

    /// Returns a mutable reference to the node with the given id or `None` if there is no such node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::{Tree, NodeId};
    /// let mut tree = Tree::default();
    /// let id = tree.add_value(2);
    ///
    /// if let Some(node) = tree.get_mut(id) {
    ///     node.expanded = Some(true);
    /// }
    ///
    /// assert_eq!(tree.get(id).map(|node| node.is_expanded()), Some(true));
    /// ```
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node<T>> {
        self.nodes.get_mut(id.index0())
    }

    /// Returns the id of the given `Node` or `None` if there is no such node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::{Tree, Node, NodeId};
    /// let mut tree = Tree::default();
    /// let r = tree.add_value(0);
    /// let a = tree.add_value(1);
    /// let b = tree.add_value(2);
    ///
    /// let node = &tree[a];
    ///
    /// assert_eq!(tree.get_id(node), Some(a));
    /// ```
    pub fn get_id(&self, node: &Node<T>) -> Option<NodeId> {
        let range = self.nodes.as_ptr_range();
        let p = node as *const Node<T>;

        if !range.contains(&p) {
            return None;
        }

        let idx = (p as usize - range.start as usize) / std::mem::size_of::<Node<T>>();
        Some(NodeId::new(idx))
    }

    pub fn root(&self) -> Option<&Node<T>> {
        self.nodes.first()
    }

    pub fn root_mut(&mut self) -> Option<&mut Node<T>> {
        self.nodes.first_mut()
    }

    pub fn empty(&mut self) {
        self.nodes.drain(..);
    }

    /// Creates a node with the given value and adds it to the tree. Returns the [`NodeId`] assigned
    /// to it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let id = tree.add_value(2);
    ///
    /// assert_eq!(*tree[id], 2);
    /// ```
    pub fn add_value(&mut self, value: T) -> NodeId {
        let node = Node::new(value);
        self.add_node(node)
    }

    /// Adds the given node to the tree. Returns the [`NodeId`] assigned to it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::{Tree, Node};
    /// let mut tree = Tree::default();
    /// let id = tree.add_node(Node::new(5));
    ///
    /// assert_eq!(*tree[id], 5);
    /// ```
    pub fn add_node(&mut self, node: Node<T>) -> NodeId {
        let id = NodeId::new(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// Sets [new](NodeId) as [parent](NodeId)'s last child.
    ///
    /// # Examples
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// tree.append(r, a);
    /// tree.append(r, b);
    /// let mut iter = tree.children(r);
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn append(&mut self, parent: NodeId, new: NodeId) {
        self.on_insert(new, Some(parent), self[parent].last_child, None);
    }

    /// Sets [new](NodeId) as [parent](NodeId)'s first child.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    ///
    /// tree.prepend(r, a);
    /// tree.prepend(r, b);
    ///
    /// let mut iter = tree.children(r);
    ///
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), Some(a));
    ///
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn prepend(&mut self, parent: NodeId, new: NodeId) {
        self.on_insert(new, Some(parent), None, self[parent].first_child);
    }

    /// Inserts [new](NodeId) after [sibling](NodeId).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// tree.append(r, a);
    /// tree.append(r, b);
    /// tree.insert_after(a, c);
    ///
    /// let mut iter = tree.children(r);
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(c));
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn insert_after(&mut self, sibling: NodeId, new: NodeId) {
        let (parent, next) = {
            let sibling = &self[sibling];
            (sibling.parent, sibling.next_sibling)
        };

        self.on_insert(new, parent, Some(sibling), next);
    }

    /// Inserts [new](NodeId) before [sibling](NodeId).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// tree.append(r, a);
    /// tree.append(r, b);
    /// tree.insert_before(a, c);
    ///
    /// let mut iter = tree.children(r);
    /// assert_eq!(iter.next(), Some(c));
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn insert_before(&mut self, sibling: NodeId, new: NodeId) {
        let (parent, prev) = {
            let sibling = &self[sibling];
            (sibling.parent, sibling.previous_sibling)
        };

        self.on_insert(new, parent, prev, Some(sibling));
    }

    /// Detaches a node from its parent and siblings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// tree.append(r, a);
    /// tree.append(r, b);
    /// tree.append(r, c);
    /// tree.detach(b);
    ///
    /// assert!(tree[b].parent().is_none());
    /// assert!(tree[b].next_sibling().is_none());
    /// assert!(tree[b].previous_sibling().is_none());
    /// ```
    pub fn detach(&mut self, id: NodeId) {
        let node = &mut self[id];
        let parent = node.parent.take();
        let prev = node.previous_sibling.take();
        let next = node.next_sibling.take();

        self.update_neighbours(parent, prev, next);
    }

    /// Returns an iterator over the ids of the nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_e = tree.add_value("e");
    /// let b_f = tree.add_value("f");
    /// let a_d_g = tree.add_value("g");
    /// tree.append(r, a);
    /// tree.append(a, a_d);
    /// tree.append(a_d, a_d_g);
    /// tree.append(a, a_e);
    /// tree.append(r, b);
    /// tree.append(b, b_f);
    /// tree.append(r, c);
    ///
    /// let mut iter = tree.iter_ids();
    /// assert_eq!(iter.next(), Some(r));
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(a_d));
    /// assert_eq!(iter.next(), Some(a_d_g));
    /// assert_eq!(iter.next(), Some(a_e));
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), Some(b_f));
    /// assert_eq!(iter.next(), Some(c));
    ///
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter_ids(&self) -> impl Iterator<Item = NodeId> {
        self.descendants(NodeId::root())
            .collect::<Vec<NodeId>>()
            .into_iter()
    }

    /// Returns an iterator over the nodes.
    pub fn iter_nodes(&self) -> impl Iterator<Item = &Node<T>> {
        self.iter_ids().map(|id| &self[id])
    }

    /// Executes the given closure for each node excluding root.
    pub fn apply_to_nodes<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Node<T>),
    {
        self.iter_ids().skip(1).for_each(|id| {
            f(&mut self[id]);
        });
    }

    /// Returns an iterator over [parent](NodeId)'s children.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// let b_d = tree.add_value("d");
    /// let b_e = tree.add_value("e");
    /// tree.append(r, a);
    /// tree.append(r, b);
    /// tree.append(b, b_d);
    /// tree.append(b, b_e);
    /// tree.append(r, c);
    ///
    /// let mut iter = tree.children(r);
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), Some(c));
    /// assert_eq!(iter.next(), None);
    ///
    /// let mut iter = tree.children(b);
    /// assert_eq!(iter.next(), Some(b_d));
    /// assert_eq!(iter.next(), Some(b_e));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn children(&self, parent: NodeId) -> Children<T> {
        Children::new(parent, self)
    }

    /// Returns an iterator over the given node's ancestors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(r, b);
    ///
    /// let mut iter = tree.ancestors(a_d);
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(r));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn ancestors(&self, node: NodeId) -> Ancestors<T> {
        skip_first!(Ancestors::new(node, self))
    }

    /// Returns an iterator over the given node's predecessors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(r, b);
    ///
    /// let mut iter = tree.predecessors(a_d);
    /// assert_eq!(iter.next(), Some(a_c));
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(r));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn predecessors(&self, node: NodeId) -> Predecessors<T> {
        skip_first!(Predecessors::new(node, self))
    }

    /// Returns an iterator over the given node's following siblings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_e = tree.add_value("e");
    /// tree.append(r, a);
    /// tree.append(a, a_d);
    /// tree.append(a, a_e);
    /// tree.append(r, b);
    /// tree.append(r, c);
    ///
    /// let mut iter = tree.following_siblings(a);
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), Some(c));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn following_siblings(&self, node: NodeId) -> FollowingSiblings<T> {
        FollowingSiblings::new(node, self)
    }

    /// Returns an iterator over the given node's preceding siblings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    /// let a_e = tree.add_value("e");
    /// tree.append(r, a);
    /// tree.append(a, a_d);
    /// tree.append(a, a_e);
    /// tree.append(r, b);
    /// tree.append(r, c);
    ///
    /// let mut iter = tree.preceding_siblings(a);
    /// assert_eq!(iter.next(), None);
    ///
    /// let mut iter = tree.preceding_siblings(c);
    /// assert_eq!(iter.next(), Some(b));
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn preceding_siblings(&self, node: NodeId) -> PrecedingSiblings<T> {
        PrecedingSiblings::new(node, self)
    }

    /// Returns an iterator over the given node's descendants including itself.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bawa::tree::Tree;
    /// let mut tree = Tree::default();
    ///
    /// let r = tree.add_value("r");
    /// let a = tree.add_value("a");
    /// let b = tree.add_value("b");
    /// let a_c = tree.add_value("c");
    /// let a_d = tree.add_value("d");
    ///
    /// tree.append(r, a);
    /// tree.append(a, a_c);
    /// tree.append(a, a_d);
    /// tree.append(r, b);
    ///
    /// let mut iter = tree.descendants(a);
    ///
    /// assert_eq!(iter.next(), Some(a));
    /// assert_eq!(iter.next(), Some(a_c));
    /// assert_eq!(iter.next(), Some(a_d));
    ///
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn descendants(&self, ancestor: NodeId) -> Descendants<T> {
        Descendants::new(ancestor, self)
    }
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::default(),
        }
    }
}

impl<T> Index<NodeId> for Tree<T> {
    type Output = Node<T>;

    fn index(&self, index: NodeId) -> &Self::Output {
        &self.nodes[index.index0()]
    }
}

impl<T> IndexMut<NodeId> for Tree<T> {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        &mut self.nodes[index.index0()]
    }
}

impl<T> Index<usize> for Tree<T> {
    type Output = Node<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

impl<T> IndexMut<usize> for Tree<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.nodes[index]
    }
}

#[cfg(test)]
mod tests {
    use crate::tree::Tree;

    #[test]
    fn swap_nodes() {
        let mut tree = Tree::default();

        let r = tree.add_value("r");
        let a = tree.add_value("a");
        let b = tree.add_value("b");
        let c = tree.add_value("c");

        tree.append(r, a);
        tree.append(r, b);
        tree.append(r, c);

        tree.detach(a);
        tree.insert_after(c, a);

        let mut iter = tree.children(r);

        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.next(), Some(c));
        assert_eq!(iter.next(), Some(a));

        assert_eq!(iter.next(), None);
    }
}
