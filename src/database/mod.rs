use crate::collection::{Collection, CollectionOpenError, CollectionOpenOptions};
use crate::common::GenerationId;
use crate::config::Config;
use crate::context::Context;
use crate::protos::database_meta::CollectionRecord;
use crate::raw_db::{RawDb, RawDbError};
use protobuf::Message;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Database {
    meta_raw_db: Arc<RawDb>,
    collections: Arc<std::sync::RwLock<HashMap<String, Collection>>>,
}

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

pub enum GetReaderGenerationIdFnError {
    NoSuchCollection,
    NoSuchReader,
}

pub struct DatabaseInner {
    collections: Arc<std::sync::RwLock<HashMap<String, Collection>>>,
}

impl DatabaseInner {
    pub fn get_reader_generation_id(
        &self,
        collection_id: &str,
        reader_id: &str,
    ) -> Result<GenerationId, GetReaderGenerationIdFnError> {
        let collections = self.collections.read().unwrap();
        todo!();
        Err(GetReaderGenerationIdFnError::NoSuchCollection)
    }
}

impl Database {
    pub async fn open(options: DatabaseOpenOptions) -> Result<Self, DatabaseOpenError> {
        let config = options.config;
        let data_path = config.data_path.clone();
        let meta_raw_db = options.meta_raw_db.clone();

        let collection_records = meta_raw_db
            .get_range(b"collection:", b"collection;")
            .await
            .or_else(|err| Err(DatabaseOpenError::RawDb(err)))?;

        let mut collections_arc = Arc::new(std::sync::RwLock::new(HashMap::new()));
        let mut collections = collections_arc.write().unwrap();

        let database_inner = Arc::new(DatabaseInner {
            collections: collections_arc.clone(),
        });

        for (key, value) in collection_records {
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
            meta_raw_db,
            collections: collections_arc.clone(),
        })
    }
}
