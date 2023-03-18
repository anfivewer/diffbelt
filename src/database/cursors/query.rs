use crate::common::{OwnedGenerationId, OwnedPhantomId};
use crate::raw_db::query_collection_records::LastAndNextRecordKey;
use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};
use rand::RngCore;
use std::borrow::Borrow;
use std::collections::HashMap;
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
    cursor: Arc<QueryCursor>,
    is_current: bool,
}

pub enum QueryCursorRef {
    Cursor(QueryCursorRefCursor),
    Empty,
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

    pub final_public_id: Option<QueryCursorPublicId>,
    pub current_cursor: Option<Arc<QueryCursor>>,
    pub next_cursor: Option<Arc<QueryCursor>>,
}

pub struct InnerQueryCursors {
    pub cursors: IndexedContainer<InnerQueryCursor>,
    pub public_ids: HashMap<QueryCursorPublicId, InnerQueryCursorId>,
}

pub enum QueryCursorError {
    NoSuchCollection,
    NoSuchCursor,
    AlreadyFinished,
    NotYetFinished,
}

impl InnerQueryCursors {
    pub fn new() -> Self {
        Self {
            cursors: IndexedContainer::new(),
            public_ids: HashMap::new(),
        }
    }

    pub fn add_cursor(&mut self, data: AddQueryCursorData) -> QueryCursorPublicId {
        let AddQueryCursorData {
            generation_id,
            phantom_id,
            last_and_next_record_key,
        } = data;

        let public_id = self.generate_public_id();

        let inner_id = self.cursors.insert(|inner_id| InnerQueryCursor {
            inner_id,
            final_public_id: None,
            current_cursor: None,
            next_cursor: Some(Arc::new(QueryCursor {
                public_id,
                generation_id,
                phantom_id,
                last_and_next_record_key,
            })),
        });

        self.public_ids.insert(public_id, inner_id);

        public_id
    }

    pub fn cursor_by_public_id(
        &self,
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
                return Some((inner_id.clone(), QueryCursorRef::Empty));
            }
        }

        None
    }

    pub fn add_cursor_continuation(
        &mut self,
        inner_id: &InnerQueryCursorId,
        data: AddQueryCursorContinuationData,
    ) -> Result<QueryCursorPublicId, QueryCursorError> {
        let AddQueryCursorContinuationData {
            last_and_next_record_key,
        } = data;

        let public_id = self.generate_public_id();

        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(QueryCursorError::NoSuchCursor);
        };

        let Some(next_cursor) = &cursor.next_cursor else {
            return Err(QueryCursorError::AlreadyFinished);
        };

        let generation_id = next_cursor.generation_id.clone();
        let phantom_id = next_cursor.phantom_id.clone();

        self.public_ids.insert(public_id, inner_id.clone());

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
            self.public_ids.remove(&old_public_id.0);
        }

        Ok(public_id)
    }

    pub fn finish_cursor(
        &mut self,
        inner_id: &InnerQueryCursorId,
    ) -> Result<QueryCursorPublicId, QueryCursorError> {
        let public_id = self.generate_public_id();

        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(QueryCursorError::NoSuchCursor);
        };

        let Some(_) = &cursor.next_cursor else {
            return Err(QueryCursorError::AlreadyFinished);
        };

        self.public_ids.insert(public_id, inner_id.clone());

        let mut old_next_cursor = cursor.next_cursor.take();

        mem::swap(&mut cursor.current_cursor, &mut old_next_cursor);

        let old_current_cursor = old_next_cursor;

        if let Some(old_current_cursor) = old_current_cursor {
            let old_public_id = old_current_cursor.public_id;

            // TODO: only mark that is was removed, remove only after some time?
            //       this can prevent buggy code that still tries to fetch old cursors
            //       and with low probability can fetch brand new one
            self.public_ids.remove(&old_public_id.0);
        }

        cursor.final_public_id.replace(public_id);

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
        let Some(_) = &cursor.current_cursor else {
            return Ok(());
        };

        cursor.current_cursor.take();

        Ok(())
    }

    fn generate_public_id(&self) -> QueryCursorPublicId {
        let mut rng = rand::thread_rng();

        let public_id = loop {
            let id = rng.next_u64();
            if self.public_ids.contains_key(&id) {
                continue;
            }

            break id;
        };

        QueryCursorPublicId(public_id)
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
