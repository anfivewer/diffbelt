use crate::common::OwnedCollectionKey;

use crate::raw_db::RawDbError;

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
