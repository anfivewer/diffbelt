use crate::common::OwnedCollectionKey;
use crate::raw_db::diff_collection_records::state::in_memory::InMemoryChangedKeysIter;
use crate::raw_db::diff_collection_records::state::single_generation::SingleGenerationChangedKeysIter;
use crate::raw_db::RawDbError;

pub enum ChangedKeysIteratorImpl<'a> {
    InMemory(InMemoryChangedKeysIter),
    SingleGeneration(SingleGenerationChangedKeysIter<'a>),
}

impl<'a> Iterator for ChangedKeysIteratorImpl<'a> {
    type Item = Result<OwnedCollectionKey, RawDbError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ChangedKeysIteratorImpl::InMemory(iter) => iter.next(),
            ChangedKeysIteratorImpl::SingleGeneration(iter) => iter.next(),
        }
    }
}
