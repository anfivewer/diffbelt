use crate::collection::open::{CollectionOpenError, CollectionOpenOptions};
use crate::collection::Collection;

use crate::database::config::DatabaseConfig;
use crate::database::constants::DATABASE_RAW_DB_CF;
use crate::database::{Database, DatabaseInner};
use crate::protos::database_meta::CollectionRecord;
use crate::raw_db::{RawDb, RawDbError, RawDbOptions};
use protobuf::Message;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct DatabaseOpenOptions<'a> {
    pub data_path: &'a PathBuf,
    pub config: Arc<DatabaseConfig>,
}

#[derive(Debug)]
pub enum DatabaseOpenError {
    CollectionOpen(CollectionOpenError),
    RawDb(RawDbError),
    CollectionsReading,
    CollectionRawDbDeletion(std::io::Error),
    JoinError,
}

impl Database {
    pub async fn open(options: DatabaseOpenOptions<'_>) -> Result<Self, DatabaseOpenError> {
        let data_path = options.data_path;

        let meta_raw_db_path = data_path.join("_meta");
        let meta_raw_db_path = meta_raw_db_path.to_str().unwrap();

        let meta_raw_db = RawDb::open_raw_db(RawDbOptions {
            path: meta_raw_db_path,
            comparator: None,
            column_families: vec![],
        })
        .expect("Cannot open meta raw_db");

        let database_raw_db = Arc::new(meta_raw_db);

        let collection_records = database_raw_db
            .get_range_cf(DATABASE_RAW_DB_CF, b"collection:", b"collection;")
            .await
            .map_err(|err| DatabaseOpenError::RawDb(err))?;

        let collections_arc = Arc::new(RwLock::new(HashMap::new()));
        let mut collections_lock = collections_arc.write().await;

        let collections_for_deletion = Arc::new(RwLock::new(HashSet::new()));

        let database_inner = Arc::new(DatabaseInner::new(
            collections_for_deletion.clone(),
            database_raw_db.clone(),
            collections_arc.clone(),
        ));

        for (_, value) in collection_records {
            let record = CollectionRecord::parse_from_bytes(&value)
                .or(Err(DatabaseOpenError::CollectionsReading))?;

            let id = record.id;

            let is_deleted = database_inner
                .is_marked_for_deletion_sync(id.as_str())
                .map_err(|err| DatabaseOpenError::RawDb(err))?;

            if is_deleted {
                let path = Collection::get_path(data_path, &id);
                std::fs::remove_dir_all(path).or_else(|err| {
                    match err.kind() {
                        std::io::ErrorKind::NotFound => {
                            return Ok(());
                        }
                        _ => {}
                    }

                    Err(DatabaseOpenError::CollectionRawDbDeletion(err))
                })?;

                let database_inner = database_inner.clone();
                database_inner
                    .finish_delete_collection_sync(&id)
                    .map_err(|err| DatabaseOpenError::RawDb(err))?;

                continue;
            }

            let collection = Collection::open(CollectionOpenOptions {
                config: options.config.clone(),
                name: id.clone(),
                data_path,
                is_manual: record.is_manual,
                database_inner: database_inner.clone(),
            })
            .await
            .or_else(|err| Err(DatabaseOpenError::CollectionOpen(err)))?;

            collections_lock.insert(id, collection);
        }

        Ok(Database {
            config: options.config,
            data_path: data_path.clone(),
            database_raw_db,
            collections_for_deletion,
            collections_alter_lock: Mutex::new(()),
            collections: collections_arc.clone(),
            inner: database_inner,
        })
    }
}
