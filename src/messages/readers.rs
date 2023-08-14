use crate::common::collection::CollectionName;
use crate::common::reader::ReaderName;
use crate::common::OwnedGenerationId;
use crate::database::DatabaseInner;
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

pub struct ReaderNewCollectionTaskResponse {
    pub minimum_generation_id: watch::Receiver<OwnedGenerationId>,
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
}

pub struct UpdateReadersTask {
    pub updates: Vec<UpdateReaderTask>,
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

pub enum DatabaseCollectionReadersTask {
    Init(Arc<DatabaseInner>),
    NewCollection(ReaderNewCollectionTask),
    UpdateReader(UpdateReaderTask),
    UpdateReaders(UpdateReadersTask),
    DeleteReader(DeleteReaderTask),
    GetReadersPointingToCollectionExceptThisOne(GetReadersPointingToCollectionTask),
    InitFinish,
    Finish,
}
