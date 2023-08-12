use crate::collection::CommitGenerationUpdateReader;
use crate::common::OwnedGenerationId;
use crate::database::generations::collection::{
    GenerationIdNextGenerationIdPair, InnerGenerationsCollectionId,
};
use crate::database::generations::next_generation_lock::GenerationIdLock;
use crate::database::DatabaseInner;
use crate::raw_db::{RawDb, RawDbError};
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

pub struct NewCollectionGenerationsTaskResponse {
    pub collection_id: InnerGenerationsCollectionId,
    pub generation_pair_receiver: watch::Receiver<GenerationIdNextGenerationIdPair>,
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
    pub next_generation_id: OwnedGenerationId,
    pub lock: GenerationIdLock,
}

pub enum LockManualGenerationIdError {
    GenerationIdMismatch,
    PutPhantomWithoutGenerationId,
}

pub struct LockNextGenerationIdTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender:
        oneshot::Sender<Result<LockNextGenerationIdTaskResponse, LockManualGenerationIdError>>,
    // Required for manual collections and phantoms
    pub next_generation_id: Option<OwnedGenerationId>,
    pub is_phantom: bool,
}

pub struct LockGenerationIdTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<GenerationIdLock>,
}

pub enum StartManualGenerationIdError {
    OutdatedGeneration,
    RawDb(RawDbError),
}

pub struct StartManualGenerationIdTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<Result<(), StartManualGenerationIdError>>,
    pub next_generation_id: OwnedGenerationId,
    pub abort_outdated: bool,
}

pub enum CommitManualGenerationError {
    OutdatedGeneration,
    RawDb(RawDbError),
}

pub struct CommitManualGenerationTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<Result<(), CommitManualGenerationError>>,

    pub generation_id: OwnedGenerationId,
    pub update_readers: Option<Vec<CommitGenerationUpdateReader>>,
}

pub struct AbortManualGenerationTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<Result<(), CommitManualGenerationError>>,
    pub generation_id: OwnedGenerationId,
}

pub enum DatabaseCollectionGenerationsTask {
    Init(Arc<DatabaseInner>),
    NewCollection(NewCollectionGenerationsTask),
    DropCollection(DropCollectionGenerationsTask),

    LockGenerationId(LockGenerationIdTask),
    LockNextGenerationId(LockNextGenerationIdTask),
    StartManualGenerationId(StartManualGenerationIdTask),
    AbortManualGeneration(AbortManualGenerationTask),
    CommitManualGeneration(CommitManualGenerationTask),
}
