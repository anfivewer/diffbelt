use crate::collection::newgen::{NewGenerationCommiter, NewGenerationCommiterOptions};
use crate::collection::util::generation_key_compare::generation_key_compare_fn;
use crate::collection::util::meta_merge::{meta_full_merge, meta_partial_merge};
use crate::collection::util::phantom_key_compare::phantom_key_compare_fn;
use crate::collection::util::record_key_compare::record_key_compare_fn;
use crate::collection::Collection;
use crate::common::{IsByteArray, IsByteArrayMut, NeverEq, OwnedGenerationId};

use crate::collection::util::generation_size_merge::{
    generation_size_full_merge, generation_size_partial_merge,
};
use crate::database::config::DatabaseConfig;
use crate::database::DatabaseInner;
use crate::raw_db::{
    RawDb, RawDbColumnFamily, RawDbComparator, RawDbError, RawDbMerge, RawDbOpenError, RawDbOptions,
};
use crate::util::bytes::increment;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::sync::{oneshot, RwLock};

pub struct CollectionOpenOptions<'a> {
    pub config: Arc<DatabaseConfig>,
    pub id: String,
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
    KeyCreation,
    DbContainsInvalidKeys,
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
        let collection_id = options.id;

        let path = options.data_path.join(&collection_id);
        let path = path.to_str().ok_or(CollectionOpenError::PathJoin)?;

        let raw_db = RawDb::open_raw_db(RawDbOptions {
            path,
            comparator: Some(RawDbComparator {
                name: "v1".to_string(),
                compare_fn: record_key_compare_fn,
            }),
            column_families: vec![
                RawDbColumnFamily {
                    name: "gens".to_string(),
                    comparator: Some(RawDbComparator {
                        name: "v1".to_string(),
                        compare_fn: generation_key_compare_fn,
                    }),
                    merge: None,
                },
                RawDbColumnFamily {
                    name: "gens_size".to_string(),
                    comparator: None,
                    merge: Some(RawDbMerge {
                        name: "v1".to_string(),
                        full_merge: Box::new(generation_size_full_merge),
                        partial_merge: Box::new(generation_size_partial_merge),
                    }),
                },
                RawDbColumnFamily {
                    name: "phantoms".to_string(),
                    comparator: Some(RawDbComparator {
                        name: "v1".to_string(),
                        compare_fn: phantom_key_compare_fn,
                    }),
                    merge: None,
                },
                RawDbColumnFamily {
                    name: "meta".to_string(),
                    comparator: None,
                    merge: Some(RawDbMerge {
                        name: "v1".to_string(),
                        full_merge: Box::new(meta_full_merge),
                        partial_merge: Box::new(meta_partial_merge),
                    }),
                },
            ],
        })?;

        let meta_raw_db = raw_db.with_cf("meta");

        let is_manual_stored = meta_raw_db.get(b"is_manual").await?;
        let is_manual = match is_manual_stored {
            Some(is_manual_vec) => {
                if is_manual_vec.len() != 1 {
                    return Err(CollectionOpenError::ManualModeMissmatch);
                }

                is_manual_vec[0] == 1
            }
            None => {
                meta_raw_db
                    .put(
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

        let generation_id_stored = meta_raw_db.get(b"generation_id").await?;
        let generation_id = match generation_id_stored {
            Some(generation_id) => OwnedGenerationId(generation_id),
            None => {
                if is_manual {
                    meta_raw_db
                        .put(b"generation_id", &vec![].into_boxed_slice())
                        .await?;

                    OwnedGenerationId(vec![].into_boxed_slice())
                } else {
                    meta_raw_db
                        .put(b"generation_id", &vec![0; 64].into_boxed_slice())
                        .await?;

                    OwnedGenerationId(vec![0; 64].into_boxed_slice())
                }
            }
        };

        let next_generation_id_stored = meta_raw_db.get(b"next_generation_id").await?;
        let next_generation_id = match next_generation_id_stored {
            Some(next_generation_id) => Some(OwnedGenerationId(next_generation_id)),
            None => {
                if is_manual {
                    None
                } else {
                    let mut next_generation_id = generation_id.clone();
                    let next_generation_id_ref = &mut next_generation_id;
                    let bytes = next_generation_id_ref.get_byte_array_mut();
                    increment(bytes);

                    let next_generation_id_cloned = next_generation_id.clone();

                    meta_raw_db
                        .put(
                            b"next_generation_id",
                            next_generation_id.as_ref().get_byte_array(),
                        )
                        .await?;

                    Some(next_generation_id_cloned)
                }
            }
        };

        let (newgen, collection_sender, on_put_sender) = if !is_manual {
            let (on_put_sender, on_put_receiver) = watch::channel(NeverEq);
            let (collection_sender, collection_receiver) = oneshot::channel();

            (
                Some(NewGenerationCommiter::new(NewGenerationCommiterOptions {
                    collection_receiver,
                    on_put_receiver,
                })),
                Some(collection_sender),
                Some(on_put_sender),
            )
        } else {
            (None, None, None)
        };

        let (generation_id_sender, generation_id_receiver) = watch::channel(generation_id.clone());

        let collection = Collection {
            config: options.config,
            id: collection_id,
            raw_db: Arc::new(raw_db),
            meta_raw_db: Arc::new(meta_raw_db),
            is_manual,
            generation_id_sender: Arc::new(generation_id_sender),
            generation_id_receiver,
            generation_id: Arc::new(RwLock::new(generation_id)),
            next_generation_id: Arc::new(RwLock::new(next_generation_id)),
            if_not_present_writes: std::sync::RwLock::new(HashMap::new()),
            database_inner: options.database_inner,
            newgen,
            on_put_sender,
            query_cursors: std::sync::RwLock::new(HashMap::new()),
            diff_cursors: std::sync::RwLock::new(HashMap::new()),
        };
        let collection = Arc::new(collection);

        match collection_sender {
            Some(collection_sender) => {
                collection_sender
                    .send(collection.clone())
                    .or(Err(()))
                    .unwrap();
            }
            None => {}
        }

        Ok(collection)
    }
}
