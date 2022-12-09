use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
use crate::collection::util::generation_key_compare::generation_key_compare_fn;
use crate::collection::util::phantom_key_compare::phantom_key_compare_fn;
use crate::collection::util::record_key_compare::record_key_compare_fn;
use crate::collection::Collection;
use crate::common::{CollectionKey, GenerationId, IsByteArray, IsByteArrayMut};
use crate::config::Config;
use crate::database::DatabaseInner;
use crate::generation::{CollectionGeneration, CollectionGenerationKeys};
use crate::raw_db::{
    RawDb, RawDbColumnFamily, RawDbComparator, RawDbError, RawDbOpenError, RawDbOptions,
};
use crate::util::bytes::increment;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CollectionOpenOptions {
    pub id: String,
    pub config: Arc<Config>,
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
    pub async fn open(options: CollectionOpenOptions) -> Result<Self, CollectionOpenError> {
        let collection_id = options.id;

        let config = options.config;
        let path = Path::new(&config.data_path).join(&collection_id);
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
                },
                RawDbColumnFamily {
                    name: "phantoms".to_string(),
                    comparator: Some(RawDbComparator {
                        name: "v1".to_string(),
                        compare_fn: phantom_key_compare_fn,
                    }),
                },
                RawDbColumnFamily {
                    name: "meta".to_string(),
                    comparator: None,
                },
            ],
        })?;

        let meta = raw_db.with_cf("meta");
        let generations = raw_db.with_cf("gens");

        let is_manual_stored = meta.get(b"is_manual").await?;
        let is_manual = match is_manual_stored {
            Some(is_manual_vec) => {
                if is_manual_vec.len() != 1 {
                    return Err(CollectionOpenError::ManualModeMissmatch);
                }

                is_manual_vec[0] == 1
            }
            None => {
                meta.put(
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

        let generation_id_stored = meta.get(b"generation_id").await?;
        let generation_id = match generation_id_stored {
            Some(generation_id) => GenerationId(generation_id),
            None => {
                if is_manual {
                    meta.put(b"generation_id", &vec![].into_boxed_slice())
                        .await?;

                    GenerationId(vec![].into_boxed_slice())
                } else {
                    meta.put(b"generation_id", &vec![0; 64].into_boxed_slice())
                        .await?;

                    GenerationId(vec![0; 64].into_boxed_slice())
                }
            }
        };

        let next_generation_id_stored = meta.get(b"next_generation_id").await?;
        let next_generation_id = match next_generation_id_stored {
            Some(next_generation_id) => Some(GenerationId(next_generation_id)),
            None => {
                if is_manual {
                    None
                } else {
                    let mut next_generation_id = generation_id.clone();
                    let next_generation_id_ref = &mut next_generation_id;
                    let bytes = next_generation_id_ref.get_byte_array_mut();
                    increment(bytes);

                    let next_generation_id_cloned = next_generation_id.clone();

                    meta.put(b"next_generation_id", next_generation_id.get_byte_array())
                        .await?;

                    Some(next_generation_id_cloned)
                }
            }
        };

        let next_generation: Option<CollectionGeneration> = match next_generation_id {
            Some(id) => {
                let empty_key = CollectionKey::empty();
                let from_key = OwnedGenerationKey::new(&id, &empty_key)
                    .or(Err(CollectionOpenError::KeyCreation))?;

                let mut to_id = id.clone();
                to_id.increment();
                let to_key = OwnedGenerationKey::new(&to_id, &empty_key)
                    .or(Err(CollectionOpenError::KeyCreation))?;

                let generation_keys = generations.get_key_range(&from_key, &to_key).await?;

                let expected_count = generation_keys.len();
                let generation_keys: Vec<CollectionKey> = generation_keys
                    .into_iter()
                    .map_while(|key| {
                        let key = GenerationKey::validate(&key).ok()?;
                        let key = key.get_key().to_owned();

                        Some(key)
                    })
                    .collect();

                if generation_keys.len() != expected_count {
                    return Err(CollectionOpenError::DbContainsInvalidKeys);
                }

                let set: BTreeSet<CollectionKey> = generation_keys.into_iter().collect();

                Some(CollectionGeneration {
                    id,
                    keys: CollectionGenerationKeys::InProgress(std::sync::RwLock::new(set)),
                })
            }
            None => None,
        };

        Ok(Collection {
            id: collection_id,
            raw_db: Arc::new(raw_db),
            is_manual,
            generation_id: std::sync::RwLock::new(generation_id),
            next_generation: RwLock::new(next_generation),
            database_inner: options.database_inner,
        })
    }
}
