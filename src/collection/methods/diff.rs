use crate::collection::cursor::diff::get_pack::GetPackOptions;
use crate::collection::cursor::diff::{DiffCursor, DiffCursorNewOptions, DiffCursorPack};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{KeyValueDiff, OwnedGenerationId};

use crate::collection::cursor::util::save_next_cursor;
use crate::common::generation_id::GenerationIdSource;
use std::sync::Arc;

type CursorId = String;
type NextCursorId = String;

pub struct DiffOptions {
    pub from_generation_id: GenerationIdSource,
    pub to_generation_id_loose: Option<OwnedGenerationId>,
}

pub struct ReadDiffCursorOptions {
    pub cursor_id: CursorId,
}

pub struct DiffOk {
    pub from_generation_id: Option<OwnedGenerationId>,
    pub to_generation_id: OwnedGenerationId,
    pub items: Vec<KeyValueDiff>,
    pub cursor_id: Option<NextCursorId>,
}

impl Collection {
    pub async fn diff(&self, options: DiffOptions) -> Result<DiffOk, CollectionMethodError> {
        let DiffOptions {
            from_generation_id,
            to_generation_id_loose,
        } = options;

        let to_generation_id_loose = {
            match to_generation_id_loose {
                Some(id) => id,
                None => {
                    let generation_id_lock = self.generation_id.read().await;
                    generation_id_lock.as_ref().to_owned()
                }
            }
        };

        let cursor = Arc::new(DiffCursor::new(DiffCursorNewOptions {
            from_generation_id,
            to_generation_id_loose,
            omit_intermediate_values: true,
        }));

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = {
            let cursor = cursor.clone();
            let db = self.raw_db.clone();
            let db_inner = self.database_inner.clone();
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || {
                cursor.get_pack_sync(GetPackOptions {
                    this_cursor_id: None,
                    db,
                    db_inner,
                    config,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let DiffCursorPack {
            from_generation_id,
            to_generation_id,
            items,
            next_cursor,
        } = result;

        let next_cursor_id = save_next_cursor(&self.diff_cursors, &cursor, next_cursor);

        drop(deletion_lock);

        Ok(DiffOk {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id: next_cursor_id,
        })
    }

    pub async fn read_diff_cursor(
        &self,
        options: ReadDiffCursorOptions,
    ) -> Result<DiffOk, CollectionMethodError> {
        let cursor_id = options.cursor_id;

        let cursor = {
            let cursors_lock = self.diff_cursors.read().unwrap();
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
            let db_inner = self.database_inner.clone();
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || {
                cursor.get_pack_sync(GetPackOptions {
                    this_cursor_id: Some(cursor_id),
                    db,
                    db_inner,
                    config,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let DiffCursorPack {
            from_generation_id,
            to_generation_id,
            items,
            next_cursor,
        } = result;

        let next_cursor_id = save_next_cursor(&self.diff_cursors, cursor.as_ref(), next_cursor);

        drop(deletion_lock);

        Ok(DiffOk {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id: next_cursor_id,
        })
    }
}
