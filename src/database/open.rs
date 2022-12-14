use crate::collection::open::{CollectionOpenError, CollectionOpenOptions};
use crate::collection::Collection;

use crate::database::{Database, DatabaseInner};
use crate::protos::database_meta::CollectionRecord;
use crate::raw_db::{RawDb, RawDbError, RawDbOptions};
use protobuf::Message;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DatabaseOpenOptions<'a> {
    pub data_path: &'a PathBuf,
}

#[derive(Debug)]
pub enum DatabaseOpenError {
    CollectionOpen(CollectionOpenError),
    RawDb(RawDbError),
    CollectionsReading,
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

        let meta_raw_db = Arc::new(meta_raw_db);

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
                data_path,
                is_manual: record.is_manual,
                database_inner: database_inner.clone(),
            })
            .await
            .or_else(|err| Err(DatabaseOpenError::CollectionOpen(err)))?;

            collections.insert(id, collection);
        }

        Ok(Database {
            data_path: data_path.clone(),
            meta_raw_db,
            collections_alter_lock: Mutex::new(()),
            collections: collections_arc.clone(),
            inner: database_inner,
        })
    }
}
