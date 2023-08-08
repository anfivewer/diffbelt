use crate::collection::CommitGenerationUpdateReader;
use crate::common::OwnedGenerationId;
use crate::database::generations::collection::InnerGenerationsCollectionId;
use crate::database::generations::next_generation_lock::NextGenerationIdLock;
use crate::database::DatabaseInner;
use crate::raw_db::{RawDb, RawDbError};
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

pub struct NewCollectionGenerationsTaskResponse {
    pub collection_id: InnerGenerationsCollectionId,
    pub generation_id_receiver: watch::Receiver<OwnedGenerationId>,
}

pub struct NewCollectionGenerationsTask {
    pub is_manual: bool,
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
    pub db: Arc<RawDb>,
    pub sender: oneshot::Sender<NewCollectionGenerationsTaskResponse>,
}

pub struct DropCollectionGenerationsTask {
    pub collection_id: InnerGenerationsCollectionId,
}

pub struct LockNextGenerationIdTaskResponse {
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
    pub lock: NextGenerationIdLock,
}

pub struct LockNextGenerationIdTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<LockNextGenerationIdTaskResponse>,
}

pub enum CommitManualGenerationError {
    GenerationIdMissmatch,
    RawDb(RawDbError),
}

pub struct CommitManualGenerationTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<Result<(), CommitManualGenerationError>>,

    pub expected_generation_id: Option<OwnedGenerationId>,
    pub update_readers: Option<Vec<CommitGenerationUpdateReader>>,
}

pub enum DatabaseCollectionGenerationsTask {
    Init(Arc<DatabaseInner>),
    NewCollection(NewCollectionGenerationsTask),
    DropCollection(DropCollectionGenerationsTask),

    LockNextGenerationId(LockNextGenerationIdTask),
    CommitManualGeneration(CommitManualGenerationTask),
}
