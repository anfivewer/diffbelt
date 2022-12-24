use crate::collection::util::record_key::{OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKey};
use crate::common::{
    CollectionKey, GenerationId, IsByteArray, KeyValueDiff, OwnedCollectionKey,
    OwnedCollectionValue, PhantomId,
};

use crate::raw_db::diff_collection_records::state::{
    DiffState, DiffStateInMemoryMode, PrevDiffState,
};
use crate::raw_db::diff_collection_records::{DiffCollectionRecordsResult, DiffCursorState};
use crate::raw_db::RawDbError;
use rocksdb::{Direction, IteratorMode};
use std::collections::btree_set::IntoIter as BTreeSetIntoIter;
use std::collections::BTreeSet;

pub struct InMemoryChangedKeysIter {
    iterator: BTreeSetIntoIter<OwnedCollectionKey>,
}

impl InMemoryChangedKeysIter {
    pub fn new(changed_keys: BTreeSet<OwnedCollectionKey>) -> Self {
        Self {
            iterator: changed_keys.into_iter(),
        }
    }
}

impl Iterator for InMemoryChangedKeysIter {
    type Item = Result<OwnedCollectionKey, RawDbError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|item| Ok(item))
    }
}
