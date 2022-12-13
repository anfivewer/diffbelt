use crate::collection::newgen::NewGenerationCommiter;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{NeverEq, OwnedGenerationId};
use crate::database::DatabaseInner;
use crate::raw_db::RawDb;
use if_not_present::ConcurrentPutStatus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

mod if_not_present;
pub mod methods;
mod newgen;
pub mod open;
pub mod util;

pub struct Collection {
    id: String,
    raw_db: Arc<RawDb>,
    meta_raw_db: Arc<RawDb>,
    is_manual: bool,
    generation_id: Arc<std::sync::RwLock<OwnedGenerationId>>,
    next_generation_id: Arc<std::sync::RwLock<Option<OwnedGenerationId>>>,
    if_not_present_writes: std::sync::RwLock<HashMap<OwnedRecordKey, ConcurrentPutStatus>>,
    database_inner: Arc<DatabaseInner>,
    // Not defined for manual collections
    newgen: Option<NewGenerationCommiter>,
    on_put_sender: Option<watch::Sender<NeverEq>>,
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
        _reader_id: &str,
    ) -> Result<OwnedGenerationId, GetReaderGenerationIdError> {
        Ok(OwnedGenerationId(vec![].into_boxed_slice()))
    }
}
