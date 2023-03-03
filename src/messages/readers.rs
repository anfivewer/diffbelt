use crate::common::OwnedGenerationId;
use crate::database::DatabaseInner;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct UpdateReaderTask {
    pub owner_collection_name: Arc<str>,
    /** None means "not changed" */
    pub to_collection_name: Option<Arc<str>>,
    pub reader_name: Arc<str>,
    pub generation_id: Arc<OwnedGenerationId>,
}

pub struct DeleteReaderTask {
    pub owner_collection_name: Arc<str>,
    pub reader_name: Arc<str>,
}

pub struct CollectionNameReaderName {
    pub owner_collection_name: Arc<str>,
    pub reader_name: Arc<str>,
}

pub struct GetReadersPointingToCollectionTask {
    pub collection_name: Arc<str>,
    pub sender: oneshot::Sender<Vec<CollectionNameReaderName>>,
}

pub enum DatabaseCollectionReadersTask {
    Init(Arc<DatabaseInner>),
    UpdateReader(UpdateReaderTask),
    DeleteReader(DeleteReaderTask),
    GetReadersPointingToCollectionExceptThisOne(GetReadersPointingToCollectionTask),
    InitFinish,
    Finish,
}
