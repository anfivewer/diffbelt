use crate::collection::Collection;
use crate::common::GenerationId;
use crate::raw_db::RawDb;
use std::collections::HashMap;
use std::sync::Arc;

pub mod open;

pub struct Database {
    meta_raw_db: Arc<RawDb>,
    collections: Arc<std::sync::RwLock<HashMap<String, Collection>>>,
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
