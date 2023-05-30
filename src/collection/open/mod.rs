mod init_readers;

use crate::collection::newgen::{NewGenerationCommiter, NewGenerationCommiterOptions};
use crate::collection::util::generation_key_compare::generation_key_compare_fn;
use crate::collection::util::meta_merge::{meta_full_merge, meta_partial_merge};
use crate::collection::util::phantom_key_compare::phantom_key_compare_fn;
use crate::collection::util::record_key_compare::record_key_compare_fn;
use crate::collection::Collection;
use crate::common::{IsByteArray, IsByteArrayMut, NeverEq, OwnedGenerationId, OwnedPhantomId};

use crate::collection::constants::{
    COLLECTION_CF_GENERATIONS, COLLECTION_CF_GENERATIONS_SIZE, COLLECTION_CF_META,
    COLLECTION_CF_PHANTOMS,
};
use crate::collection::open::init_readers::init_readers;
use crate::collection::util::generation_size_merge::{
    generation_size_full_merge, generation_size_partial_merge,
};
use crate::database::config::DatabaseConfig;
use crate::database::DatabaseInner;
use crate::messages::cursors::{
    DatabaseCollectionCursorsTask, DropCollectionCursorsTask, NewCollectionCursorsTask,
};
use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, DropCollectionGenerationsTask, NewCollectionGenerationsTask,
};
use crate::raw_db::{
    RawDb, RawDbColumnFamily, RawDbComparator, RawDbError, RawDbMerge, RawDbOpenError, RawDbOptions,
};
use crate::util::async_spawns::{run_when_watch_is_true_or_end, watch_is_true_or_end};
use crate::util::async_sync_call::async_sync_call;
use crate::util::bytes::increment;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::pin;
use tokio::sync::watch;
use tokio::sync::{oneshot, RwLock};

pub struct CollectionOpenOptions<'a> {
    pub config: Arc<DatabaseConfig>,
    pub name: String,
    pub data_path: &'a PathBuf,
    pub is_manual: bool,
    pub database_inner: Arc<DatabaseInner>,
}

#[derive(Debug)]
pub enum CollectionOpenError {
    PathJoin,
    RawDbOpen(RawDbOpenError),
    RawDb(RawDbError),
    ManualModeMissmatch,
    InvalidGenerationId,
    InvalidPhantomId,
    JoinError,
    InvalidUtf8,
    InvalidReaderValue,
    OneshotRecv(oneshot::error::RecvError),
}

impl From<RawDbOpenError> for CollectionOpenError {
    fn from(err: RawDbOpenError) -> Self {
        CollectionOpenError::RawDbOpen(err)
    }
}

impl From<RawDbError> for CollectionOpenError {
    fn from(err: RawDbError) -> Self {
        CollectionOpenError::RawDb(err)
    }
}

impl Collection {
    // Create a new column family descriptor with the specified name and options.
    pub async fn open(
        options: CollectionOpenOptions<'_>,
    ) -> Result<Arc<Self>, CollectionOpenError> {
        let collection_name = options.name;

        let path = Collection::get_path(options.data_path, &collection_name);
        let path = path.to_str().ok_or(CollectionOpenError::PathJoin)?;

        let raw_db = RawDb::open_raw_db(RawDbOptions {
            path,
            comparator: Some(RawDbComparator {
                name: "v1".to_string(),
                compare_fn: record_key_compare_fn,
            }),
            column_families: vec![
                RawDbColumnFamily {
                    name: COLLECTION_CF_GENERATIONS.to_string(),
                    comparator: Some(RawDbComparator {
                        name: "v1".to_string(),
                        compare_fn: generation_key_compare_fn,
                    }),
                    merge: None,
                },
                RawDbColumnFamily {
                    name: COLLECTION_CF_GENERATIONS_SIZE.to_string(),
                    comparator: None,
                    merge: Some(RawDbMerge {
                        name: "v1".to_string(),
                        full_merge: Box::new(generation_size_full_merge),
                        partial_merge: Box::new(generation_size_partial_merge),
                    }),
                },
                RawDbColumnFamily {
                    name: COLLECTION_CF_PHANTOMS.to_string(),
                    comparator: Some(RawDbComparator {
                        name: "v1".to_string(),
                        compare_fn: phantom_key_compare_fn,
                    }),
                    merge: None,
                },
                RawDbColumnFamily {
                    name: COLLECTION_CF_META.to_string(),
                    comparator: None,
                    merge: Some(RawDbMerge {
                        name: "v1".to_string(),
                        full_merge: Box::new(meta_full_merge),
                        partial_merge: Box::new(meta_partial_merge),
                    }),
                },
            ],
        })?;

        let is_manual_stored = raw_db.get_cf(COLLECTION_CF_META, b"is_manual").await?;
        let is_manual = match is_manual_stored {
            Some(is_manual_vec) => {
                if is_manual_vec.len() != 1 {
                    return Err(CollectionOpenError::ManualModeMissmatch);
                }

                is_manual_vec[0] == 1
            }
            None => {
                raw_db
                    .put_cf(
                        COLLECTION_CF_META,
                        b"is_manual",
                        &vec![if options.is_manual { 1 } else { 0 }].into_boxed_slice(),
                    )
                    .await?;

                options.is_manual
            }
        };

        if is_manual != options.is_manual {
            return Err(CollectionOpenError::ManualModeMissmatch);
        }

        let generation_id_stored = raw_db.get_cf(COLLECTION_CF_META, b"generation_id").await?;
        let generation_id = match generation_id_stored {
            Some(generation_id) => OwnedGenerationId::from_boxed_slice(generation_id)
                .or(Err(CollectionOpenError::InvalidGenerationId))?,
            None => {
                if is_manual {
                    raw_db
                        .put_cf(
                            COLLECTION_CF_META,
                            b"generation_id",
                            &vec![].into_boxed_slice(),
                        )
                        .await?;

                    OwnedGenerationId::empty()
                } else {
                    raw_db
                        .put_cf(
                            COLLECTION_CF_META,
                            b"generation_id",
                            &vec![0; 8].into_boxed_slice(),
                        )
                        .await?;

                    OwnedGenerationId::zero_64bits()
                }
            }
        };

        let next_generation_id_stored = raw_db
            .get_cf(COLLECTION_CF_META, b"next_generation_id")
            .await?;
        let next_generation_id = match next_generation_id_stored {
            Some(next_generation_id) => Some(
                OwnedGenerationId::from_boxed_slice(next_generation_id)
                    .or(Err(CollectionOpenError::InvalidGenerationId))?,
            ),
            None => {
                if is_manual {
                    None
                } else {
                    let mut next_generation_id = generation_id.clone();
                    let next_generation_id_ref = &mut next_generation_id;
                    let bytes = next_generation_id_ref.get_byte_array_mut();
                    increment(bytes);

                    let next_generation_id_cloned = next_generation_id.clone();

                    raw_db
                        .put_cf(
                            COLLECTION_CF_META,
                            b"next_generation_id",
                            next_generation_id.as_ref().get_byte_array(),
                        )
                        .await?;

                    Some(next_generation_id_cloned)
                }
            }
        };

        let prev_phantom_id_stored = raw_db
            .get_cf(COLLECTION_CF_META, b"prev_phantom_id")
            .await?;
        let prev_phantom_id = match prev_phantom_id_stored {
            Some(prev_phantom_id) => OwnedPhantomId::from_boxed_slice(prev_phantom_id)
                .map_err(|_| CollectionOpenError::InvalidPhantomId)?,
            None => OwnedPhantomId::zero_64bits(),
        };

        let (newgen, collection_sender, on_put_sender) = if !is_manual {
            let (on_put_sender, on_put_receiver) = watch::channel(NeverEq);
            let (collection_sender, collection_receiver) = oneshot::channel();

            (
                Some(
                    NewGenerationCommiter::new(NewGenerationCommiterOptions {
                        collection_receiver,
                        on_put_receiver,
                    })
                    .await,
                ),
                Some(collection_sender),
                Some(on_put_sender),
            )
        } else {
            (None, None, None)
        };

        let (generation_id_sender, generation_id_receiver) = watch::channel(generation_id.clone());

        let database_inner = options.database_inner;

        let cursors_id = async_sync_call(|sender| {
            database_inner.add_cursors_task(DatabaseCollectionCursorsTask::NewCollection(
                NewCollectionCursorsTask { sender },
            ))
        })
        .await
        .map_err(CollectionOpenError::OneshotRecv)?;

        let generations_id = async_sync_call(|sender| {
            database_inner.add_generations_task(DatabaseCollectionGenerationsTask::NewCollection(
                NewCollectionGenerationsTask { sender },
            ))
        })
        .await
        .map_err(CollectionOpenError::OneshotRecv)?;

        let newgen = Arc::new(RwLock::new(newgen));

        let drop_sender = {
            let database_inner = database_inner.clone();
            let (sender, mut receiver) = oneshot::channel();

            let mut db_stop_receiver = database_inner.stop_receiver();

            let newgen = newgen.clone();
            run_when_watch_is_true_or_end(db_stop_receiver.clone(), async move {
                let mut newgen = newgen.write().await;
                if let Some(mut newgen) = newgen.take() {
                    newgen.stop().await;
                }
            });

            tokio::spawn(async move {
                let on_db_stop = watch_is_true_or_end(&mut db_stop_receiver);
                pin!(on_db_stop);

                tokio::select! {
                    _ = &mut receiver => {},
                    _ = &mut on_db_stop => {},
                }

                database_inner
                    .add_cursors_task(DatabaseCollectionCursorsTask::DropCollection(
                        DropCollectionCursorsTask {
                            collection_id: cursors_id,
                        },
                    ))
                    .await;

                database_inner
                    .add_generations_task(DatabaseCollectionGenerationsTask::DropCollection(
                        DropCollectionGenerationsTask {
                            collection_id: generations_id,
                        },
                    ))
                    .await;
            });

            sender
        };

        let collection = Collection {
            config: options.config,
            name: Arc::from(collection_name),
            raw_db: Arc::new(raw_db),
            is_manual,
            is_deleted: Arc::new(RwLock::new(false)),
            generation_id_sender: Arc::new(generation_id_sender),
            generation_id_receiver,
            generation_id: Arc::new(RwLock::new(generation_id)),
            next_generation_id: Arc::new(RwLock::new(next_generation_id)),
            if_not_present_writes: Arc::new(RwLock::new(HashMap::new())),
            database_inner,
            newgen,
            on_put_sender,
            prev_phantom_id: RwLock::new(prev_phantom_id),
            cursors_id,
            generations_id,
            drop_sender: Some(drop_sender),
        };

        let collection = Arc::new(collection);

        init_readers(collection.clone()).await?;

        match collection_sender {
            Some(collection_sender) => {
                collection_sender.send(collection.clone()).unwrap_or(());
            }
            None => {}
        }

        Ok(collection)
    }

    pub fn get_path(data_path: &PathBuf, collection_name: &str) -> PathBuf {
        data_path.join(collection_name)
    }
}
