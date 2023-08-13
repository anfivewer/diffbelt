use crate::collection::open::{CollectionOpenError, CollectionOpenOptions};
use crate::collection::Collection;

use crate::collection::methods::errors::CollectionMethodError;
use crate::database::config::DatabaseConfig;
use crate::database::constants::DATABASE_RAW_DB_CF;
use crate::database::cursors::start_cursors_task_thread;
use crate::database::generations::start_generations_task_thread;
use crate::database::readers::start_readers_task_thread;
use crate::database::{Database, DatabaseInner};
use crate::messages::generations::DatabaseCollectionGenerationsTask;
use crate::messages::readers::DatabaseCollectionReadersTask;
use crate::protos::database_meta::CollectionRecord;
use crate::raw_db::{RawDb, RawDbError, RawDbOptions};
use crate::util::async_spawns::run_when_watch_is_true_or_end;
use crate::util::atomic_cleanup::AtomicCleanup;
use protobuf::Message;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{watch, Mutex, RwLock};

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
    CollectionMethod(CollectionMethodError),
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

        let readers = start_readers_task_thread().await;
        let cursors = start_cursors_task_thread(options.config.clone()).await;
        let generations = start_generations_task_thread().await;

        let (stop_sender, stop_receiver) = watch::channel(false);

        let database_inner = Arc::new(DatabaseInner::new(
            options.config.clone(),
            collections_for_deletion.clone(),
            database_raw_db.clone(),
            collections_arc.clone(),
            readers,
            cursors,
            generations,
            stop_receiver.clone(),
        ));

        database_inner
            .add_readers_task(DatabaseCollectionReadersTask::Init(database_inner.clone()))
            .await;

        database_inner
            .add_generations_task(DatabaseCollectionGenerationsTask::Init(
                database_inner.clone(),
            ))
            .await;

        let mut deleted_collections = Vec::new();

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

                deleted_collections.push(Arc::from(id));

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

        drop(collections_lock);

        for collection_name in deleted_collections {
            let _: () = database_inner
                .remove_readers_pointing_to_collection(Arc::clone(&collection_name))
                .await
                .map_err(DatabaseOpenError::CollectionMethod)?;

            let _: () = database_inner
                .finish_delete_collection_sync(&collection_name)
                .map_err(|err| DatabaseOpenError::RawDb(err))?;
        }

        database_inner
            .add_readers_task(DatabaseCollectionReadersTask::InitFinish)
            .await;

        {
            let collections_arc = collections_arc.clone();
            run_when_watch_is_true_or_end(stop_receiver, async move {
                let mut collections = collections_arc.write().await;
                collections.clear();
            });
        }

        Ok(Database {
            config: options.config,
            data_path: data_path.clone(),
            database_raw_db,
            collections_alter_lock: Mutex::new(()),
            collections: collections_arc,
            inner: database_inner,
            stop_sender: AtomicCleanup::some(stop_sender),
        })
    }
}
