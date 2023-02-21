use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::collection::cursor::query::get_pack::GetPackOptions;
use crate::collection::cursor::query::{QueryCursor, QueryCursorNewOptions, QueryCursorPack};
use crate::common::{KeyValue, OwnedGenerationId, OwnedPhantomId};

use crate::collection::cursor::util::save_next_cursor;
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
        let generation_id = self.generation_id_or_current(options.generation_id).await;

        let cursor = Arc::new(QueryCursor::new(QueryCursorNewOptions {
            generation_id: generation_id.clone(),
            phantom_id: options.phantom_id,
        }));

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = {
            let cursor = cursor.clone();
            let db = self.raw_db.clone();
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || {
                cursor.get_pack_sync(GetPackOptions {
                    this_cursor_id: None,
                    db,
                    config,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let QueryCursorPack { items, next_cursor } = result;

        let next_cursor_id = save_next_cursor(&self.query_cursors, &cursor, next_cursor);

        drop(deletion_lock);

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

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = {
            let cursor = cursor.clone();
            let db = self.raw_db.clone();
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || {
                cursor.get_pack_sync(GetPackOptions {
                    this_cursor_id: Some(cursor_id),
                    db,
                    config,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let QueryCursorPack { items, next_cursor } = result;

        let next_cursor_id = save_next_cursor(&self.query_cursors, cursor.as_ref(), next_cursor);

        drop(deletion_lock);

        Ok(QueryOk {
            generation_id: cursor.get_generation_id().to_owned(),
            items,
            cursor_id: next_cursor_id,
        })
    }
}
