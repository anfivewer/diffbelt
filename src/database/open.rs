use crate::collection::open::{CollectionOpenError, CollectionOpenOptions};
use crate::collection::Collection;
use crate::config::Config;
use crate::database::{Database, DatabaseInner};
use crate::protos::database_meta::CollectionRecord;
use crate::raw_db::{RawDb, RawDbError};
use protobuf::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DatabaseOpenOptions {
    pub config: Arc<Config>,
    pub meta_raw_db: Arc<RawDb>,
}

#[derive(Debug)]
pub enum DatabaseOpenError {
    CollectionOpen(CollectionOpenError),
    RawDb(RawDbError),
    CollectionsReading,
}

impl Database {
    pub async fn open(options: DatabaseOpenOptions) -> Result<Self, DatabaseOpenError> {
        let config = options.config;
        let meta_raw_db = options.meta_raw_db.clone();

        let collection_records = meta_raw_db
            .get_range(b"collection:", b"collection;")
            .await
            .or_else(|err| Err(DatabaseOpenError::RawDb(err)))?;

        let collections_arc = Arc::new(std::sync::RwLock::new(HashMap::new()));
        let mut collections = collections_arc.write().unwrap();

        let database_inner = Arc::new(DatabaseInner {
            collections: collections_arc.clone(),
        });

        for (_, value) in collection_records {
            let record = CollectionRecord::parse_from_bytes(&value)
                .or(Err(DatabaseOpenError::CollectionsReading))?;

            let id = record.id;

            let collection = Collection::open(CollectionOpenOptions {
                id: id.clone(),
                config: config.clone(),
                is_manual: record.is_manual,
                database_inner: database_inner.clone(),
            })
            .await
            .or_else(|err| Err(DatabaseOpenError::CollectionOpen(err)))?;

            collections.insert(id, collection);
        }

        Ok(Database {
            config,
            meta_raw_db,
            collections_alter_lock: Mutex::new(()),
            collections: collections_arc.clone(),
            inner: database_inner,
        })
    }
}
