use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::collection::cursor::query::get_pack::GetPackOptions;
use crate::collection::cursor::query::{QueryCursor, QueryCursorNewOptions, QueryCursorPack};
use crate::common::{KeyValue, OwnedGenerationId, OwnedPhantomId};
use crate::util::base62::rand_b62;

use std::sync::Arc;

type CursorId = String;
type NextCursorId = String;

pub struct QueryOptions {
    pub generation_id: Option<OwnedGenerationId>,
    pub phantom_id: Option<OwnedPhantomId>,
}

pub struct ReadQueryCursorOptions {
    pub cursor_id: CursorId,
}

pub struct QueryOk {
    pub generation_id: OwnedGenerationId,
    pub items: Vec<KeyValue>,
    pub cursor_id: Option<NextCursorId>,
}

impl Collection {
    pub async fn query(&self, options: QueryOptions) -> Result<QueryOk, CollectionMethodError> {
        let generation_id = match options.generation_id {
            Some(gen) => gen,
            None => {
                let generation_id_lock = self.generation_id.read().await;
                generation_id_lock.as_ref().to_owned()
            }
        };

        let cursor = Arc::new(QueryCursor::new(QueryCursorNewOptions {
            generation_id: generation_id.clone(),
            phantom_id: options.phantom_id,
        }));

        let result = {
            let cursor = cursor.clone();
            let db = self.raw_db.clone();
            tokio::task::spawn_blocking(move || {
                cursor.get_pack_sync(GetPackOptions {
                    this_cursor_id: None,
                    db,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let QueryCursorPack { items, next_cursor } = result;

        let next_cursor_id = self.save_next_cursor(&cursor, next_cursor);

        Ok(QueryOk {
            generation_id,
            items,
            cursor_id: next_cursor_id,
        })
    }

    pub async fn read_query_cursor(
        &self,
        options: ReadQueryCursorOptions,
    ) -> Result<QueryOk, CollectionMethodError> {
        let cursor_id = options.cursor_id;

        let cursor = {
            let cursors_lock = self.query_cursors.read().unwrap();
            let cursor = cursors_lock
                .get(&cursor_id)
                .ok_or(CollectionMethodError::NoSuchCursor)?;
            cursor.clone()
        };

        let result = {
            let cursor = cursor.clone();
            let db = self.raw_db.clone();
            tokio::task::spawn_blocking(move || {
                cursor.get_pack_sync(GetPackOptions {
                    this_cursor_id: Some(cursor_id),
                    db,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let QueryCursorPack { items, next_cursor } = result;

        let next_cursor_id = self.save_next_cursor(cursor.as_ref(), next_cursor);

        Ok(QueryOk {
            generation_id: cursor.get_generation_id().to_owned(),
            items,
            cursor_id: next_cursor_id,
        })
    }

    fn save_next_cursor(
        &self,
        current_cursor: &QueryCursor,
        next_cursor: Option<QueryCursor>,
    ) -> Option<NextCursorId> {
        match next_cursor {
            Some(next_cursor) => {
                let next_cursor = Arc::new(next_cursor);

                let mut cursors_lock = self.query_cursors.write().unwrap();
                let mut id;

                loop {
                    id = rand_b62(11);
                    if !cursors_lock.contains_key(&id) {
                        break;
                    }
                }

                cursors_lock.insert(id.clone(), next_cursor.clone());

                match current_cursor.get_prev_cursor_id() {
                    Some(prev_cursor_id) => {
                        // if current cursor was accessed, we can drop previous one
                        cursors_lock.remove(prev_cursor_id);
                    }
                    None => {}
                }

                Some(id)
            }
            None => match current_cursor.get_prev_cursor_id() {
                Some(prev_cursor_id) => {
                    let mut cursors_lock = self.query_cursors.write().unwrap();
                    cursors_lock.remove(prev_cursor_id);
                    None
                }
                None => None,
            },
        }
    }
}
