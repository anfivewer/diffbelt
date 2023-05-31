use crate::common::OwnedGenerationId;
use crate::database::generations::collection::InnerGenerationsCollectionId;
use crate::database::DatabaseInner;
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

pub struct NewCollectionGenerationsTaskResponse {
    pub collection_id: InnerGenerationsCollectionId,
    pub generation_id_receiver: watch::Receiver<OwnedGenerationId>,
}

pub struct NewCollectionGenerationsTask {
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
    pub sender: oneshot::Sender<NewCollectionGenerationsTaskResponse>,
}

pub struct DropCollectionGenerationsTask {
    pub collection_id: InnerGenerationsCollectionId,
}

pub struct LockNextGenerationIdTaskResponse {
    //
}

pub struct LockNextGenerationIdTask {
    pub collection_id: InnerGenerationsCollectionId,
    pub sender: oneshot::Sender<LockNextGenerationIdTaskResponse>,
}

pub enum DatabaseCollectionGenerationsTask {
    Init(Arc<DatabaseInner>),
    NewCollection(NewCollectionGenerationsTask),
    DropCollection(DropCollectionGenerationsTask),

    LockNextGenerationId(LockNextGenerationIdTask),
}
