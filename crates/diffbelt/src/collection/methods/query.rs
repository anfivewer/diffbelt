use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use std::marker::PhantomData;

use crate::collection::cursor::query::get_pack::GetPackOptions;
use crate::collection::cursor::query::{QueryCursorNewOptions, QueryCursorPack};
use crate::common::{KeyValue, OwnedGenerationId, OwnedPhantomId};
use crate::database::cursors::query::{
    AddQueryCursorContinuationData, AddQueryCursorData, QueryCursor,
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

pub struct QueryOptions {
    pub generation_id: Option<OwnedGenerationId>,
    pub phantom_id: Option<OwnedPhantomId>,
}

pub struct ReadQueryCursorOptions {
    pub cursor_id: CursorId,
}

pub struct AbortQueryCursorOptions {
    pub cursor_id: CursorId,
}

pub struct QueryOk {
    pub generation_id: OwnedGenerationId,
    pub items: Vec<KeyValue>,
    pub cursor_id: Option<NextCursorId>,
}

impl Collection {
    pub async fn query(&self, options: QueryOptions) -> Result<QueryOk, CollectionMethodError> {
        let QueryOptions {
            generation_id,
            phantom_id,
        } = options;

        let generation_id = self.generation_id_or_current(generation_id).await;

        let initial_cursor = QueryCursor::new(QueryCursorNewOptions {
            generation_id: generation_id.clone(),
            phantom_id: phantom_id.clone(),
        });

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = {
            let db = self.raw_db.clone();
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || {
                initial_cursor.get_pack_sync(GetPackOptions { db, config })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        let QueryCursorPack {
            items,
            last_and_next_record_key,
        } = result;

        let cursor_public_id = match last_and_next_record_key {
            last_and_next_record_key @ Some(_) => {
                let generation_id = generation_id.clone();

                let id = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Query(
                            DatabaseCollectionSpecificCursorsTask::AddQueryCursor(AddCursorTask {
                                collection_id: self.cursors_id,
                                data: AddQueryCursorData {
                                    generation_id,
                                    phantom_id,
                                    last_and_next_record_key,
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

        Ok(QueryOk {
            generation_id,
            items,
            cursor_id: cursor_public_id.map(|x| x.to_b62()),
        })
    }

    pub async fn read_query_cursor(
        &self,
        options: ReadQueryCursorOptions,
    ) -> Result<QueryOk, CollectionMethodError> {
        let ReadQueryCursorOptions { cursor_id } = options;

        let public_id = CursorPublicId::from_b62(cursor_id.as_ref())
            .map_err(|_| CollectionMethodError::NoSuchCursor)?;

        let (inner_id, cursor) = async_sync_call(|sender| {
            self.database_inner
                .add_cursors_task(DatabaseCollectionCursorsTask::Query(
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
                from_generation_id: _,
                to_generation_id: generation_id,
            }) => {
                let _ = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Query(
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

                return Ok(QueryOk {
                    generation_id,
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
            let config = self.config.clone();
            tokio::task::spawn_blocking(move || cursor.get_pack_sync(GetPackOptions { db, config }))
                .await
                .or(Err(CollectionMethodError::TaskJoin))??
        };

        let QueryCursorPack {
            items,
            last_and_next_record_key,
        } = result;

        let cursor_public_id = match last_and_next_record_key {
            last_and_next_record_key @ Some(_) => {
                let cursor_public_id = async_sync_call(|sender| {
                    self.database_inner
                        .add_cursors_task(DatabaseCollectionCursorsTask::Query(
                            DatabaseCollectionSpecificCursorsTask::AddQueryCursorContinuation(
                                AddCursorContinuationTask {
                                    collection_id: self.cursors_id,
                                    inner_id,
                                    is_current,
                                    data: AddQueryCursorContinuationData {
                                        last_and_next_record_key,
                                    },
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
                        .add_cursors_task(DatabaseCollectionCursorsTask::Query(
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

        Ok(QueryOk {
            generation_id: cursor.generation_id.clone(),
            items,
            cursor_id: cursor_public_id.map(|x| x.to_b62()),
        })
    }

    pub async fn abort_query_cursor(
        &self,
        options: AbortQueryCursorOptions,
    ) -> Result<(), CollectionMethodError> {
        let AbortQueryCursorOptions { cursor_id } = options;

        let public_id = CursorPublicId::from_b62(&cursor_id)
            .map_err(|_| CollectionMethodError::NoSuchCursor)?;

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let _ = async_sync_call(|sender| {
            self.database_inner
                .add_cursors_task(DatabaseCollectionCursorsTask::Query(
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
