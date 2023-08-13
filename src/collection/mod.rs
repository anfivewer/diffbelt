use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{OwnedGenerationId, OwnedPhantomId};
use crate::database::config::DatabaseConfig;
use crate::database::cursors::collection::InnerCursorsCollectionId;
use crate::database::generations::collection::{
    GenerationIdNextGenerationIdPair, InnerGenerationsCollectionId,
};
use crate::database::DatabaseInner;
use crate::messages::garbage_collector::NewCollectionTaskResponse;
use crate::raw_db::RawDbError;
use if_not_present::ConcurrentPutStatus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, watch, RwLock};

pub mod constants;
mod cursor;
pub mod cursors;
mod drop;
mod if_not_present;
pub mod methods;
pub mod open;
pub mod readers;
pub mod util;

pub struct Collection {
    config: Arc<DatabaseConfig>,
    name: Arc<str>,
    raw_db: CollectionRawDb,
    is_manual: bool,
    // you need to lock it for reading before any operations with raw_db
    is_deleted: Arc<RwLock<bool>>,
    pub generation_pair_receiver: watch::Receiver<GenerationIdNextGenerationIdPair>,
    if_not_present_writes: Arc<RwLock<HashMap<OwnedRecordKey, ConcurrentPutStatus>>>,
    database_inner: Arc<DatabaseInner>,
    prev_phantom_id: RwLock<OwnedPhantomId>,
    cursors_id: InnerCursorsCollectionId,
    generations_id: InnerGenerationsCollectionId,
    gc: NewCollectionTaskResponse,
    drop_sender: Option<oneshot::Sender<()>>,
}

pub enum GetReaderGenerationIdError {
    NoSuchReader,
    RawDb(RawDbError),
}

impl Collection {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn is_manual(&self) -> bool {
        self.is_manual
    }

    pub fn generation_pair(&self) -> GenerationIdNextGenerationIdPair {
        self.generation_pair_receiver.borrow().clone()
    }

    pub fn get_reader_generation_id(
        &self,
        reader_name: &str,
    ) -> Result<Option<OwnedGenerationId>, GetReaderGenerationIdError> {
        let state = self
            .raw_db
            .get_reader_sync(reader_name)
            .map_err(|err| match err {
                RawDbError::NoSuchReader => GetReaderGenerationIdError::NoSuchReader,
                err => GetReaderGenerationIdError::RawDb(err),
            })?;

        Ok(state.generation_id)
    }
}

pub struct CommitGenerationUpdateReader {
    pub reader_name: String,
    pub generation_id: OwnedGenerationId,
}
