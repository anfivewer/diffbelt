use crate::common::{OwnedGenerationId, OwnedPhantomId};
use crate::database::config::DatabaseConfig;
use crate::raw_db::query_collection_records::LastAndNextRecordKey;
use crate::util::base62;
use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};
use lru::LruCache;
use rand::RngCore;
use std::borrow::Borrow;
use std::mem;
use std::sync::Arc;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct QueryCursorPublicId(pub u64);

impl Borrow<u64> for QueryCursorPublicId {
    fn borrow(&self) -> &u64 {
        &self.0
    }
}

#[derive(Copy, Clone)]
pub struct InnerQueryCursorId {
    pub index: usize,
    pub counter: u64,
}

pub struct QueryCursor {
    pub public_id: QueryCursorPublicId,
    pub generation_id: OwnedGenerationId,
    pub phantom_id: Option<OwnedPhantomId>,
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

pub struct QueryCursorRefCursor {
    pub cursor: Arc<QueryCursor>,
    pub is_current: bool,
}

pub struct QueryCursorRefEmpty {
    pub generation_id: OwnedGenerationId,
}

pub enum QueryCursorRef {
    Cursor(QueryCursorRefCursor),
    Empty(QueryCursorRefEmpty),
}

pub struct AddQueryCursorData {
    pub generation_id: OwnedGenerationId,
    pub phantom_id: Option<OwnedPhantomId>,
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

pub struct AddQueryCursorContinuationData {
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

pub struct InnerQueryCursor {
    pub inner_id: InnerQueryCursorId,
    pub generation_id: OwnedGenerationId,

    pub final_public_id: Option<QueryCursorPublicId>,
    pub current_cursor: Option<Arc<QueryCursor>>,
    pub next_cursor: Option<Arc<QueryCursor>>,
}

type PublicIdsMap = LruCache<QueryCursorPublicId, InnerQueryCursorId>;

struct ReservedPublicId<'a> {
    public_ids: &'a mut PublicIdsMap,
    id: QueryCursorPublicId,
}

impl<'a> ReservedPublicId<'a> {
    fn generate(public_ids: &'a mut PublicIdsMap) -> Self {
        let mut rng = rand::thread_rng();

        let id = loop {
            let id = rng.next_u64();
            if public_ids.contains(&id) {
                continue;
            }

            break id;
        };

        Self {
            public_ids,
            id: QueryCursorPublicId(id),
        }
    }

    fn insert(&mut self, inner_id: InnerQueryCursorId) -> Option<InnerQueryCursorId> {
        self.public_ids.push(self.id, inner_id).map(|(_, x)| x)
    }

    fn insert_and_evict(
        &mut self,
        inner_id: InnerQueryCursorId,
        cursors: &mut IndexedContainer<InnerQueryCursor>,
    ) {
        let Some((_, evicted_inner_id)) = self.public_ids.push(self.id, inner_id) else {
            return;
        };

        cursors.delete(&evicted_inner_id);
    }
}

pub struct InnerQueryCursors {
    pub cursors: IndexedContainer<InnerQueryCursor>,
    pub public_ids: PublicIdsMap,
}

#[derive(Debug)]
pub enum QueryCursorError {
    NoSuchCollection,
    NoSuchCursor,
    AlreadyFinished,
    NotYetFinished,
}

impl InnerQueryCursors {
    pub fn new(config: &DatabaseConfig) -> Self {
        Self {
            cursors: IndexedContainer::new(),
            public_ids: LruCache::new(config.max_cursors_per_collection),
        }
    }

    pub fn add_cursor(&mut self, data: AddQueryCursorData) -> QueryCursorPublicId {
        let AddQueryCursorData {
            generation_id,
            phantom_id,
            last_and_next_record_key,
        } = data;

        let mut public_id = ReservedPublicId::generate(&mut self.public_ids);

        let inner_id = self.cursors.insert(|inner_id| InnerQueryCursor {
            inner_id,
            generation_id: generation_id.clone(),
            final_public_id: None,
            current_cursor: None,
            next_cursor: Some(Arc::new(QueryCursor {
                public_id: public_id.id,
                generation_id,
                phantom_id,
                last_and_next_record_key,
            })),
        });

        public_id.insert_and_evict(inner_id, &mut self.cursors);

        public_id.id
    }

    pub fn cursor_by_public_id(
        &mut self,
        public_id: QueryCursorPublicId,
    ) -> Option<(InnerQueryCursorId, QueryCursorRef)> {
        let Some(inner_id) = self.public_ids.get(&public_id.0) else {
            return None;
        };

        let Some(cursor) = self.cursors.get(inner_id) else {
            return None;
        };

        if let Some(cursor) = &cursor.next_cursor {
            if cursor.public_id == public_id {
                return Some((
                    inner_id.clone(),
                    QueryCursorRef::Cursor(QueryCursorRefCursor {
                        cursor: cursor.clone(),
                        is_current: false,
                    }),
                ));
            }
        }

        if let Some(cursor) = &cursor.current_cursor {
            if cursor.public_id == public_id {
                return Some((
                    inner_id.clone(),
                    QueryCursorRef::Cursor(QueryCursorRefCursor {
                        cursor: cursor.clone(),
                        is_current: true,
                    }),
                ));
            }
        }

        if let Some(id) = &cursor.final_public_id {
            if id == &public_id {
                return Some((
                    inner_id.clone(),
                    QueryCursorRef::Empty(QueryCursorRefEmpty {
                        generation_id: cursor.generation_id.clone(),
                    }),
                ));
            }
        }

        None
    }

    pub fn add_cursor_continuation(
        &mut self,
        inner_id: &InnerQueryCursorId,
        data: AddQueryCursorContinuationData,
        is_current: bool,
    ) -> Result<QueryCursorPublicId, QueryCursorError> {
        let AddQueryCursorContinuationData {
            last_and_next_record_key,
        } = data;

        let mut public_id = ReservedPublicId::generate(&mut self.public_ids);

        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(QueryCursorError::NoSuchCursor);
        };

        let Some(next_cursor) = &cursor.next_cursor else {
            return Err(QueryCursorError::AlreadyFinished);
        };

        if is_current {
            let QueryCursor {
                public_id,
                generation_id,
                phantom_id,
                last_and_next_record_key: _,
            } = next_cursor.as_ref();

            let public_id = public_id.clone();

            cursor.next_cursor.replace(Arc::new(QueryCursor {
                public_id,
                generation_id: generation_id.clone(),
                phantom_id: phantom_id.clone(),
                last_and_next_record_key,
            }));

            return Ok(public_id);
        }

        let generation_id = next_cursor.generation_id.clone();
        let phantom_id = next_cursor.phantom_id.clone();

        let evicted_inner_id = public_id.insert(inner_id.clone());
        let public_id = public_id.id;

        let mut old_next_cursor = cursor.next_cursor.replace(Arc::new(QueryCursor {
            public_id,
            generation_id,
            phantom_id,
            last_and_next_record_key,
        }));

        mem::swap(&mut cursor.current_cursor, &mut old_next_cursor);

        let old_current_cursor = old_next_cursor;

        if let Some(old_current_cursor) = old_current_cursor {
            let old_public_id = old_current_cursor.public_id;

            // TODO: only mark that is was removed, remove only after some time?
            //       this can prevent buggy code that still tries to fetch old cursors
            //       and with low probability can fetch brand new one
            self.public_ids.pop(&old_public_id.0);
        }

        if let Some(evicted_inner_id) = evicted_inner_id {
            self.cursors.delete(&evicted_inner_id);
        }

        Ok(public_id)
    }

    pub fn finish_cursor(
        &mut self,
        inner_id: &InnerQueryCursorId,
        is_current: bool,
    ) -> Result<QueryCursorPublicId, QueryCursorError> {
        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(QueryCursorError::NoSuchCursor);
        };

        if let Some(public_id) = &cursor.final_public_id {
            return Ok(public_id.clone());
        }

        let Some(_) = &cursor.next_cursor else {
            return Err(QueryCursorError::AlreadyFinished);
        };

        let mut old_next_cursor = cursor.next_cursor.take();

        let mut public_id = ReservedPublicId::generate(&mut self.public_ids);
        let evicted_inner_id = public_id.insert(inner_id.clone());
        let public_id = public_id.id;

        if is_current {
            cursor.final_public_id.replace(public_id);

            if let Some(evicted_inner_id) = evicted_inner_id {
                self.cursors.delete(&evicted_inner_id);
            }

            return Ok(public_id);
        }

        mem::swap(&mut cursor.current_cursor, &mut old_next_cursor);

        let old_current_cursor = old_next_cursor;

        if let Some(old_current_cursor) = old_current_cursor {
            let old_public_id = old_current_cursor.public_id;

            // TODO: only mark that is was removed, remove only after some time?
            //       this can prevent buggy code that still tries to fetch old cursors
            //       and with low probability can fetch brand new one
            self.public_ids.pop(&old_public_id.0);
        }

        cursor.final_public_id.replace(public_id);

        if let Some(evicted_inner_id) = evicted_inner_id {
            self.cursors.delete(&evicted_inner_id);
        }

        Ok(public_id)
    }

    pub fn fully_finish_cursor(
        &mut self,
        inner_id: &InnerQueryCursorId,
    ) -> Result<(), QueryCursorError> {
        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(QueryCursorError::NoSuchCursor);
        };

        let None = &cursor.next_cursor else {
            return Err(QueryCursorError::NotYetFinished);
        };

        let cursor = cursor.current_cursor.take();

        if let Some(cursor) = cursor {
            self.public_ids.pop(&cursor.public_id.0);
        }

        Ok(())
    }

    pub fn abort_cursor(&mut self, inner_id: &InnerQueryCursorId) -> Result<(), QueryCursorError> {
        let Some(cursor) = self.cursors.delete(inner_id) else {
            return Err(QueryCursorError::NoSuchCursor);
        };

        let InnerQueryCursor {
            inner_id: _,
            generation_id: _,
            final_public_id,
            current_cursor,
            next_cursor,
        } = cursor;

        if let Some(public_id) = final_public_id {
            self.public_ids.pop(&public_id.0);
        };

        if let Some(cursor) = current_cursor {
            self.public_ids.pop(&cursor.public_id.0);
        };

        if let Some(cursor) = next_cursor {
            self.public_ids.pop(&cursor.public_id.0);
        };

        Ok(())
    }

    #[cfg(test)]
    pub fn query_cursors_count(&self) -> usize {
        self.public_ids.len()
    }
}

impl IndexedContainerItem for InnerQueryCursor {
    type Item = InnerQueryCursor;
    type Id = InnerQueryCursorId;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        InnerQueryCursorId { index, counter }
    }
}

impl IndexedContainerPointer for InnerQueryCursorId {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl IndexedContainerPointer for InnerQueryCursor {
    fn index(&self) -> usize {
        self.inner_id.index
    }

    fn counter(&self) -> u64 {
        self.inner_id.counter
    }
}

impl QueryCursorPublicId {
    pub fn from_b62(s: &str) -> Result<Self, ()> {
        base62::to_u64(s).map(|x| Self(x))
    }
    pub fn to_b62(&self) -> Box<str> {
        base62::from_u64(self.0)
    }
}
