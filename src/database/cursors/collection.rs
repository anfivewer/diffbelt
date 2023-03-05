use crate::database::cursors::query::{InnerQueryCursor, InnerQueryCursors};
use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};

#[derive(Copy, Clone)]
pub struct InnerCursorsCollectionId {
    pub index: usize,
    pub counter: u64,
}

pub struct InnerCursorsCollection {
    pub inner_id: InnerCursorsCollectionId,
    pub query_cursors: InnerQueryCursors,
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
