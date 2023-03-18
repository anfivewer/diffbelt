use crate::database::cursors::query::InnerQueryCursors;
use crate::util::indexed_container::{IndexedContainerItem, IndexedContainerPointer};

#[derive(Copy, Clone)]
pub struct InnerCursorsCollectionId {
    pub index: usize,
    pub counter: u64,
}

pub struct InnerCursorsCollection {
    pub inner_id: InnerCursorsCollectionId,
    pub query_cursors: InnerQueryCursors,
}

impl InnerCursorsCollection {
    pub fn new(inner_id: InnerCursorsCollectionId) -> Self {
        Self {
            inner_id,
            query_cursors: InnerQueryCursors::new(),
        }
    }
}

impl IndexedContainerItem for InnerCursorsCollection {
    type Item = InnerCursorsCollection;
    type Id = InnerCursorsCollectionId;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        InnerCursorsCollectionId { index, counter }
    }
}

impl IndexedContainerPointer for InnerCursorsCollectionId {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl IndexedContainerPointer for InnerCursorsCollection {
    fn index(&self) -> usize {
        self.inner_id.index
    }

    fn counter(&self) -> u64 {
        self.inner_id.counter
    }
}
