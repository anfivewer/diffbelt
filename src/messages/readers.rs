use crate::common::collection::CollectionName;
use crate::common::reader::ReaderName;
use crate::common::OwnedGenerationId;
use crate::database::DatabaseInner;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, watch, OwnedRwLockReadGuard, RwLock};

pub struct ReaderNewCollectionTaskResponse {
    pub minimum_generation_id: watch::Receiver<OwnedGenerationId>,
    pub minimum_generation_id_lock: Arc<RwLock<()>>,
}

pub struct ReaderNewCollectionTask {
    pub collection_name: CollectionName,
    pub sender: oneshot::Sender<ReaderNewCollectionTaskResponse>,
}

pub struct UpdateReaderTask {
    pub owner_collection_name: CollectionName,
    /** None means "not changed" */
    pub to_collection_name: Option<CollectionName>,
    pub reader_name: ReaderName,
    pub generation_id: OwnedGenerationId,
    pub sender: Option<oneshot::Sender<()>>,
}

pub struct UpdateReadersTask {
    pub updates: Vec<UpdateReaderTask>,
    pub sender: oneshot::Sender<()>,
}

pub struct DeleteReaderTask {
    pub owner_collection_name: CollectionName,
    pub reader_name: ReaderName,
}

pub struct CollectionNameReaderName {
    pub owner_collection_name: CollectionName,
    pub reader_name: ReaderName,
}

pub struct GetReadersPointingToCollectionTask {
    pub collection_name: CollectionName,
    pub sender: oneshot::Sender<Vec<CollectionNameReaderName>>,
}

pub struct GetMinimumGenerationIdLocksTaskResponse {
    pub minimum_generation_ids_with_locks:
        HashMap<ReaderName, (OwnedGenerationId, OwnedRwLockReadGuard<()>)>,
}

pub struct GetMinimumGenerationIdLocksTask {
    pub collection_name: CollectionName,
    pub reader_names: Vec<ReaderName>,
    pub sender: oneshot::Sender<GetMinimumGenerationIdLocksTaskResponse>,
}

pub enum DatabaseCollectionReadersTask {
    Init(Arc<DatabaseInner>),
    NewCollection(ReaderNewCollectionTask),
    UpdateReader(UpdateReaderTask),
    UpdateReaders(UpdateReadersTask),
    DeleteReader(DeleteReaderTask),
    GetReadersPointingToCollectionExceptThisOne(GetReadersPointingToCollectionTask),
    GetMinimumGenerationIdLocks(GetMinimumGenerationIdLocksTask),
    InitFinish,
    Finish,
}
