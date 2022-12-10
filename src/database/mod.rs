use crate::collection::Collection;
use crate::common::GenerationId;
use crate::config::Config;
use crate::raw_db::RawDb;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod create_collection;
pub mod open;

pub struct Database {
    config: Arc<Config>,
    meta_raw_db: Arc<RawDb>,
    collections_alter_lock: Mutex<()>,
    collections: Arc<std::sync::RwLock<HashMap<String, Arc<Collection>>>>,
    inner: Arc<DatabaseInner>,
}

pub enum GetReaderGenerationIdFnError {
    NoSuchCollection,
    NoSuchReader,
}

pub struct DatabaseInner {
    collections: Arc<std::sync::RwLock<HashMap<String, Arc<Collection>>>>,
}

impl DatabaseInner {
    pub fn get_reader_generation_id(
        &self,
        _collection_id: &str,
        _reader_id: &str,
    ) -> Result<GenerationId, GetReaderGenerationIdFnError> {
        let _collections = self.collections.read().unwrap();
        todo!()
    }
}
