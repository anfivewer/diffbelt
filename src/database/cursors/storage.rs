use crate::common::{GenerationId, OwnedGenerationId, PhantomId};
use crate::database::config::DatabaseConfig;
use crate::util::base62;
use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};
use lru::LruCache;
use rand::RngCore;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem;
use std::sync::Arc;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct CursorPublicId(pub u64);

impl Borrow<u64> for CursorPublicId {
    fn borrow(&self) -> &u64 {
        &self.0
    }
}

pub trait CursorType: Copy {
    type Data;
    type AddData;
    type AddContinuationData;

    fn public_id_from_data(data: &Self::Data) -> CursorPublicId;
    fn phantom_id_from_data(data: &Self::Data) -> Option<PhantomId<'_>>;
    fn from_generation_id_from_add_data(data: &Self::AddData) -> Option<GenerationId<'_>>;
    fn to_generation_id_from_add_data(data: &Self::AddData) -> GenerationId<'_>;
    fn data_from_add_data(data: Self::AddData, public_id: CursorPublicId) -> Self::Data;
    fn replace_data_from_continuation(
        continuation_data: Self::AddContinuationData,
        data: &Self::Data,
    ) -> Self::Data;
    fn new_data_from_continuation(
        continuation_data: Self::AddContinuationData,
        data: &Self::Data,
        public_id: CursorPublicId,
    ) -> Self::Data;
}

#[derive(Copy, Clone)]
pub struct InnerCursorId<T: CursorType> {
    pub cursor_type: PhantomData<T>,
    pub index: usize,
    pub counter: u64,
}

pub struct CursorRefCursor<T: CursorType> {
    pub cursor: Arc<T::Data>,
    pub is_current: bool,
}

pub struct CursorRefEmpty {
    pub from_generation_id: Option<OwnedGenerationId>,
    pub to_generation_id: OwnedGenerationId,
}

pub enum CursorRef<T: CursorType> {
    Cursor(CursorRefCursor<T>),
    Empty(CursorRefEmpty),
}

pub struct InnerCursor<T: CursorType> {
    pub inner_id: InnerCursorId<T>,
    pub from_generation_id: Option<OwnedGenerationId>,
    pub to_generation_id: OwnedGenerationId,

    pub final_public_id: Option<CursorPublicId>,
    pub current_cursor: Option<Arc<T::Data>>,
    pub next_cursor: Option<Arc<T::Data>>,
}

type PublicIdsMap<T> = LruCache<CursorPublicId, InnerCursorId<T>>;

struct ReservedPublicId<'a, T: CursorType> {
    public_ids: &'a mut PublicIdsMap<T>,
    id: CursorPublicId,
}

impl<'a, T: CursorType> ReservedPublicId<'a, T> {
    fn generate(public_ids: &'a mut PublicIdsMap<T>) -> Self {
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
            id: CursorPublicId(id),
        }
    }

    fn insert(&mut self, inner_id: InnerCursorId<T>) -> Option<InnerCursorId<T>> {
        self.public_ids.push(self.id, inner_id).map(|(_, x)| x)
    }

    fn insert_and_evict(
        &mut self,
        inner_id: InnerCursorId<T>,
        cursors: &mut IndexedContainer<InnerCursor<T>>,
    ) {
        let Some((_, evicted_inner_id)) = self.public_ids.push(self.id, inner_id) else {
            return;
        };

        cursors.delete(&evicted_inner_id);
    }
}

pub struct InnerCursors<T: CursorType> {
    pub cursors: IndexedContainer<InnerCursor<T>>,
    pub public_ids: PublicIdsMap<T>,
}

#[derive(Debug)]
pub enum CursorError {
    NoSuchCollection,
    NoSuchCursor,
    AlreadyFinished,
    NotYetFinished,
}

impl<T: CursorType> InnerCursors<T> {
    pub fn new(config: &DatabaseConfig) -> Self {
        Self {
            cursors: IndexedContainer::new(),
            public_ids: LruCache::new(config.max_cursors_per_collection),
        }
    }

    pub fn add_cursor(&mut self, data: T::AddData) -> CursorPublicId {
        let mut public_id = ReservedPublicId::generate(&mut self.public_ids);

        let inner_id = self.cursors.insert(|inner_id| InnerCursor {
            inner_id,
            from_generation_id: None,
            to_generation_id: T::to_generation_id_from_add_data(&data).to_owned(),
            final_public_id: None,
            current_cursor: None,
            next_cursor: Some(Arc::new(T::data_from_add_data(data, public_id.id))),
        });

        public_id.insert_and_evict(inner_id, &mut self.cursors);

        public_id.id
    }

    pub fn cursor_by_public_id(
        &mut self,
        public_id: CursorPublicId,
    ) -> Option<(InnerCursorId<T>, CursorRef<T>)> {
        let Some(inner_id) = self.public_ids.get(&public_id.0) else {
            return None;
        };

        let Some(cursor) = self.cursors.get(inner_id) else {
            return None;
        };

        if let Some(cursor) = &cursor.next_cursor {
            if T::public_id_from_data(&cursor) == public_id {
                return Some((
                    inner_id.clone(),
                    CursorRef::Cursor(CursorRefCursor {
                        cursor: cursor.clone(),
                        is_current: false,
                    }),
                ));
            }
        }

        if let Some(cursor) = &cursor.current_cursor {
            if T::public_id_from_data(&cursor) == public_id {
                return Some((
                    inner_id.clone(),
                    CursorRef::Cursor(CursorRefCursor {
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
                    CursorRef::Empty(CursorRefEmpty {
                        from_generation_id: cursor.from_generation_id.clone(),
                        to_generation_id: cursor.to_generation_id.clone(),
                    }),
                ));
            }
        }

        None
    }

    pub fn add_cursor_continuation(
        &mut self,
        inner_id: &InnerCursorId<T>,
        continuation_data: T::AddContinuationData,
        is_current: bool,
    ) -> Result<CursorPublicId, CursorError> {
        let mut public_id = ReservedPublicId::generate(&mut self.public_ids);

        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(CursorError::NoSuchCursor);
        };

        let Some(next_cursor) = &cursor.next_cursor else {
            return Err(CursorError::AlreadyFinished);
        };

        if is_current {
            let data = next_cursor.as_ref();
            let new_data = T::replace_data_from_continuation(continuation_data, data);
            let public_id = T::public_id_from_data(&new_data);

            cursor.next_cursor.replace(Arc::new(new_data));

            return Ok(public_id);
        }

        let evicted_inner_id = public_id.insert(inner_id.clone());
        let public_id = public_id.id;

        let new_data = T::new_data_from_continuation(continuation_data, next_cursor, public_id);

        let mut old_next_cursor = cursor.next_cursor.replace(Arc::new(new_data));

        mem::swap(&mut cursor.current_cursor, &mut old_next_cursor);

        let old_current_cursor = old_next_cursor;

        if let Some(old_current_cursor) = old_current_cursor {
            let old_public_id = T::public_id_from_data(&old_current_cursor);

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
        inner_id: &InnerCursorId<T>,
        is_current: bool,
    ) -> Result<CursorPublicId, CursorError> {
        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(CursorError::NoSuchCursor);
        };

        if let Some(public_id) = &cursor.final_public_id {
            return Ok(public_id.clone());
        }

        let Some(_) = &cursor.next_cursor else {
            return Err(CursorError::AlreadyFinished);
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
            let old_public_id = T::public_id_from_data(&old_current_cursor);

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

    pub fn fully_finish_cursor(&mut self, inner_id: &InnerCursorId<T>) -> Result<(), CursorError> {
        let Some(cursor) = self.cursors.get_mut(inner_id) else {
            return Err(CursorError::NoSuchCursor);
        };

        let None = &cursor.next_cursor else {
            return Err(CursorError::NotYetFinished);
        };

        let cursor = cursor.current_cursor.take();

        if let Some(cursor) = cursor {
            self.public_ids.pop(&T::public_id_from_data(&cursor).0);
        }

        Ok(())
    }

    pub fn abort_cursor(&mut self, inner_id: &InnerCursorId<T>) -> Result<(), CursorError> {
        let Some(cursor) = self.cursors.delete(inner_id) else {
            return Err(CursorError::NoSuchCursor);
        };

        let InnerCursor {
            inner_id: _,
            from_generation_id: _,
            to_generation_id: _,
            final_public_id,
            current_cursor,
            next_cursor,
        } = cursor;

        if let Some(public_id) = final_public_id {
            self.public_ids.pop(&public_id.0);
        };

        if let Some(cursor) = current_cursor {
            self.public_ids.pop(&T::public_id_from_data(&cursor).0);
        };

        if let Some(cursor) = next_cursor {
            self.public_ids.pop(&T::public_id_from_data(&cursor).0);
        };

        Ok(())
    }

    #[cfg(test)]
    pub fn query_cursors_count(&self) -> usize {
        self.public_ids.len()
    }
}

impl<T: CursorType> IndexedContainerItem for InnerCursor<T> {
    type Item = InnerCursor<T>;
    type Id = InnerCursorId<T>;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        InnerCursorId {
            cursor_type: PhantomData::default(),
            index,
            counter,
        }
    }
}

impl<T: CursorType> IndexedContainerPointer for InnerCursorId<T> {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl<T: CursorType> IndexedContainerPointer for InnerCursor<T> {
    fn index(&self) -> usize {
        self.inner_id.index
    }

    fn counter(&self) -> u64 {
        self.inner_id.counter
    }
}

impl CursorPublicId {
    pub fn from_b62(s: &str) -> Result<Self, ()> {
        base62::to_u64(s).map(|x| Self(x))
    }
    pub fn to_b62(&self) -> Box<str> {
        base62::from_u64(self.0)
    }
}
