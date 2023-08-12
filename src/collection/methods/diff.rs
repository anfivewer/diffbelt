use crate::collection::cursor::diff::get_pack::GetPackOptions;
use crate::collection::cursor::diff::{DiffCursorNewOptions, DiffCursorPack};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{KeyValueDiff, OwnedGenerationId};
use std::marker::PhantomData;

use crate::common::generation_id::GenerationIdSource;
use crate::database::cursors::diff::{
    AddDiffCursorContinuationData, AddDiffCursorData, DiffCursor,
};
use crate::database::cursors::storage::{
    CursorPublicId, CursorRef, CursorRefCursor, CursorRefEmpty,
};
use crate::messages::cursors::{
    AbortCursorTask, AddCursorContinuationTask, AddCursorTask, DatabaseCollectionCursorsTask,
    DatabaseCollectionSpecificCursorsTask, FinishCursorTask, FullyFinishCursorTask,
    GetCursorByPublicIdTask,
};

use crate::util::async_sync_call::async_sync_call;

type CursorId = Box<str>;
type NextCursorId = Box<str>;

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

        let initial_cursor = DiffCursor::new(DiffCursorNewOptions {
            from_generation_id,
            to_generation_id_loose,
            omit_intermediate_values: true,
        });

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = {
            let db = self.raw_db.clone();
            let db_inner = self.database_inner.clone();
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || {
                initial_cursor.get_pack_sync(GetPackOptions {
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
            next_diff_state,
        } = result;

        let cursor_public_id = match next_diff_state {
            next_diff_state @ Some(_) => {
                let from_generation_id = from_generation_id.clone();
                let to_generation_id = to_generation_id.clone();

                let id = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Diff(
                            DatabaseCollectionSpecificCursorsTask::AddQueryCursor(AddCursorTask {
                                collection_id: self.cursors_id,
                                data: AddDiffCursorData {
                                    from_generation_id,
                                    to_generation_id,
                                    omit_intermediate_values: true,
                                    raw_db_cursor_state: next_diff_state,
                                },
                                sender,
                            }),
                        ))
                })
                .await
                .map_err(CollectionMethodError::OneshotRecv)?
                .map_err(CollectionMethodError::QueryCursor)?;

                Some(id)
            }
            None => None,
        };

        drop(deletion_lock);

        Ok(DiffOk {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id: cursor_public_id.map(|x| x.to_b62()),
        })
    }

    pub async fn read_diff_cursor(
        &self,
        options: ReadDiffCursorOptions,
    ) -> Result<DiffOk, CollectionMethodError> {
        let cursor_id = options.cursor_id;

        let public_id = CursorPublicId::from_b62(cursor_id.as_ref())
            .map_err(|_| CollectionMethodError::NoSuchCursor)?;

        let (inner_id, cursor) = async_sync_call(|sender| {
            self.database_inner
                .add_cursors_task(DatabaseCollectionCursorsTask::Diff(
                    DatabaseCollectionSpecificCursorsTask::GetQueryCursorByPublicId(
                        GetCursorByPublicIdTask {
                            collection_id: self.cursors_id,
                            public_id,
                            sender,
                        },
                    ),
                ))
        })
        .await
        .map_err(CollectionMethodError::OneshotRecv)?
        .ok_or_else(|| CollectionMethodError::NoSuchCursor)?;

        let cursor = match cursor {
            CursorRef::Cursor(cursor) => cursor,
            CursorRef::Empty(CursorRefEmpty {
                from_generation_id,
                to_generation_id,
            }) => {
                let _ = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Diff(
                            DatabaseCollectionSpecificCursorsTask::FullyFinishQueryCursor(
                                FullyFinishCursorTask {
                                    collection_id: self.cursors_id,
                                    inner_id,
                                    sender,
                                },
                            ),
                        ))
                })
                .await;

                return Ok(DiffOk {
                    from_generation_id,
                    to_generation_id,
                    items: vec![],
                    cursor_id: None,
                });
            }
        };

        let CursorRefCursor { cursor, is_current } = cursor;

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
            next_diff_state,
        } = result;

        let cursor_public_id = match next_diff_state {
            next_diff_state @ Some(_) => {
                let cursor_public_id = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Diff(
                            DatabaseCollectionSpecificCursorsTask::AddQueryCursorContinuation(
                                AddCursorContinuationTask {
                                    collection_id: self.cursors_id,
                                    inner_id,
                                    is_current,
                                    data: AddDiffCursorContinuationData { next_diff_state },
                                    sender,
                                },
                            ),
                        ))
                })
                .await
                .map_err(CollectionMethodError::OneshotRecv)?
                .map_err(CollectionMethodError::QueryCursor)?;

                Some(cursor_public_id)
            }
            None => {
                let cursor_public_id = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Diff(
                            DatabaseCollectionSpecificCursorsTask::FinishQueryCursor(
                                FinishCursorTask {
                                    collection_id: self.cursors_id,
                                    inner_id,
                                    is_current,
                                    sender,
                                },
                            ),
                        ))
                })
                .await
                .map_err(CollectionMethodError::OneshotRecv)?
                .map_err(CollectionMethodError::QueryCursor)?;

                Some(cursor_public_id)
            }
        };

        drop(deletion_lock);

        Ok(DiffOk {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id: cursor_public_id.map(|x| x.to_b62()),
        })
    }

    pub async fn abort_diff_cursor(
        &self,
        options: AbortDiffCursorOptions,
    ) -> Result<(), CollectionMethodError> {
        let AbortDiffCursorOptions { cursor_id } = options;

        let public_id = CursorPublicId::from_b62(&cursor_id)
            .map_err(|_| CollectionMethodError::NoSuchCursor)?;

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let _ = async_sync_call(|sender| {
            self.database_inner
                .add_cursors_task(DatabaseCollectionCursorsTask::Diff(
                    DatabaseCollectionSpecificCursorsTask::AbortQueryCursor(AbortCursorTask {
                        cursor_type: PhantomData::default(),
                        collection_id: self.cursors_id,
                        public_id,
                        sender,
                    }),
                ))
        })
        .await;

        drop(deletion_lock);

        Ok(())
    }
}
