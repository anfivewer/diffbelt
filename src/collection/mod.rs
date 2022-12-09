use crate::common::GenerationId;
use crate::database::DatabaseInner;
use crate::generation::CollectionGeneration;
use crate::raw_db::RawDb;
use std::sync::Arc;
use tokio::sync::RwLock;

mod methods;
pub mod open;
mod util;

pub struct Collection {
    id: String,
    raw_db: Arc<RawDb>,
    is_manual: bool,
    generation_id: std::sync::RwLock<GenerationId>,
    // None if this is manual collection and generation is not yet started
    // in non-manual collections always present
    next_generation: RwLock<Option<CollectionGeneration>>,
    database_inner: Arc<DatabaseInner>,
}

pub enum GetReaderGenerationIdError {
    NoSuchReader,
}

impl Collection {
    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn is_manual(&self) -> bool {
        self.is_manual
    }

    pub fn get_reader_generation_id(
        &self,
        reader_id: &str,
    ) -> Result<GenerationId, GetReaderGenerationIdError> {
        Ok(GenerationId(vec![].into_boxed_slice()))
    }
}
