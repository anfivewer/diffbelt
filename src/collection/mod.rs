use crate::collection::cursor::diff::DiffCursor;
use crate::collection::cursor::query::QueryCursor;
use crate::collection::newgen::NewGenerationCommiter;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{NeverEq, OwnedGenerationId};
use crate::database::config::DatabaseConfig;
use crate::database::DatabaseInner;
use crate::raw_db::{RawDb, RawDbError};
use if_not_present::ConcurrentPutStatus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

pub mod constants;
mod cursor;
mod if_not_present;
pub mod methods;
mod newgen;
pub mod open;
pub mod util;

pub struct Collection {
    config: Arc<DatabaseConfig>,
    id: String,
    raw_db: Arc<RawDb>,
    is_manual: bool,
    // you need to lock it for reading before any operations with raw_db
    is_deleted: RwLock<bool>,
    generation_id_sender: Arc<watch::Sender<OwnedGenerationId>>,
    generation_id_receiver: watch::Receiver<OwnedGenerationId>,
    generation_id: Arc<RwLock<OwnedGenerationId>>,
    next_generation_id: Arc<RwLock<Option<OwnedGenerationId>>>,
    if_not_present_writes: Arc<RwLock<HashMap<OwnedRecordKey, ConcurrentPutStatus>>>,
    database_inner: Arc<DatabaseInner>,
    // Not defined for manual collections
    newgen: Option<NewGenerationCommiter>,
    on_put_sender: Option<watch::Sender<NeverEq>>,
    query_cursors: std::sync::RwLock<HashMap<String, Arc<QueryCursor>>>,
    diff_cursors: std::sync::RwLock<HashMap<String, Arc<DiffCursor>>>,
}

pub enum GetReaderGenerationIdError {
    NoSuchReader,
    RawDb(RawDbError),
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
    ) -> Result<Option<OwnedGenerationId>, GetReaderGenerationIdError> {
        let state = self
            .raw_db
            .get_reader_sync(reader_id)
            .map_err(|err| match err {
                RawDbError::NoSuchReader => GetReaderGenerationIdError::NoSuchReader,
                err => GetReaderGenerationIdError::RawDb(err),
            })?;

        Ok(state.generation_id)
    }

    pub fn get_generation_id_receiver(&self) -> watch::Receiver<OwnedGenerationId> {
        self.generation_id_receiver.clone()
    }
}
