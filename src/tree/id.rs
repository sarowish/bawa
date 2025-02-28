use std::num::NonZeroUsize;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct NodeId {
    id: NonZeroUsize,
}

impl NodeId {
    pub fn new(index0: usize) -> Self {
        Self {
            id: NonZeroUsize::new(index0.wrapping_add(1)).unwrap(),
        }
    }

    pub fn index0(self) -> usize {
        self.id.get() - 1
    }
}
