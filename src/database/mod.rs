use crate::collection::{Collection, GetReaderGenerationIdError};
use crate::common::OwnedGenerationId;

use crate::database::config::DatabaseConfig;
use crate::raw_db::{RawDb, RawDbError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod config;
pub mod create_collection;
pub mod open;

pub struct Database {
    config: Arc<DatabaseConfig>,
    data_path: PathBuf,
    meta_raw_db: Arc<RawDb>,
    collections_alter_lock: Mutex<()>,
    collections: Arc<std::sync::RwLock<HashMap<String, Arc<Collection>>>>,
    inner: Arc<DatabaseInner>,
}

pub enum GetReaderGenerationIdFnError {
    NoSuchCollection,
    NoSuchReader,
    RawDb(RawDbError),
}

pub struct DatabaseInner {
    collections: Arc<std::sync::RwLock<HashMap<String, Arc<Collection>>>>,
}

impl DatabaseInner {
    pub fn get_reader_generation_id_sync(
        &self,
        collection_id: &str,
        reader_id: &str,
    ) -> Result<Option<OwnedGenerationId>, GetReaderGenerationIdFnError> {
        let collections_lock = self.collections.read().unwrap();

        let collection = collections_lock
            .get(collection_id)
            .ok_or(GetReaderGenerationIdFnError::NoSuchCollection)?;

        let collection = collection.clone();

        drop(collections_lock);

        collection
            .get_reader_generation_id(reader_id)
            .map_err(|err| match err {
                GetReaderGenerationIdError::NoSuchReader => {
                    GetReaderGenerationIdFnError::NoSuchReader
                }
                GetReaderGenerationIdError::RawDb(err) => GetReaderGenerationIdFnError::RawDb(err),
            })
    }
}
