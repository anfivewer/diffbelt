use crate::util::indexed_container::{IndexedContainerItem, IndexedContainerPointer};

#[derive(Copy, Clone)]
pub struct InnerGenerationsCollectionId {
    pub index: usize,
    pub counter: u64,
}

pub struct InnerGenerationsCollection {
    pub inner_id: InnerGenerationsCollectionId,
}

impl InnerGenerationsCollection {
    pub fn new(inner_id: InnerGenerationsCollectionId) -> Self {
        Self { inner_id }
    }
}

impl IndexedContainerItem for InnerGenerationsCollection {
    type Item = InnerGenerationsCollection;
    type Id = InnerGenerationsCollectionId;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        InnerGenerationsCollectionId { index, counter }
    }
}

impl IndexedContainerPointer for InnerGenerationsCollectionId {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl IndexedContainerPointer for InnerGenerationsCollection {
    fn index(&self) -> usize {
        self.inner_id.index
    }

    fn counter(&self) -> u64 {
        self.inner_id.counter
    }
}
