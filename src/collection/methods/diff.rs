use crate::collection::cursor::diff::get_pack::GetPackOptions;
use crate::collection::cursor::diff::{DiffCursor, DiffCursorNewOptions, DiffCursorPack};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{KeyValueDiff, OwnedGenerationId};

use crate::collection::cursor::util::{save_next_cursor, BaseCursor};
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

pub struct AbortDiffCursorOptions {
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

        let to_generation_id_loose = self.generation_id_or_current(to_generation_id_loose).await;

        let cursor = Arc::new(std::sync::RwLock::new(DiffCursor::new(
            DiffCursorNewOptions {
                from_generation_id,
                to_generation_id_loose,
                omit_intermediate_values: true,
            },
        )));

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
                let cursor = cursor.read().unwrap();
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

        let next_cursor_id = save_next_cursor(&self.diff_cursors, cursor, next_cursor);

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
                let cursor = cursor.read().unwrap();
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

        let next_cursor_id = save_next_cursor(&self.diff_cursors, cursor, next_cursor);

        drop(deletion_lock);

        Ok(DiffOk {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id: next_cursor_id,
        })
    }

    pub async fn abort_diff_cursor(
        &self,
        options: AbortDiffCursorOptions,
    ) -> Result<(), CollectionMethodError> {
        let AbortDiffCursorOptions { cursor_id } = options;

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        {
            let mut diff_cursors = self.diff_cursors.write().unwrap();

            let cursor = diff_cursors.remove(&cursor_id);
            let Some(cursor) = cursor else {
                return Ok(());
            };

            let cursor = cursor.read().unwrap();

            let prev_id = cursor.prev_cursor_id();
            let next_id = cursor.next_cursor_id();

            if let Some(id) = prev_id {
                diff_cursors.remove(id);
            }
            if let Some(id) = next_id {
                diff_cursors.remove(id);
            }
        }

        drop(deletion_lock);

        Ok(())
    }
}
