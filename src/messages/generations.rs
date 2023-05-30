use crate::database::generations::collection::InnerGenerationsCollectionId;
use crate::database::DatabaseInner;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct NewCollectionGenerationsTask {
    pub sender: oneshot::Sender<InnerGenerationsCollectionId>,
}

pub struct DropCollectionGenerationsTask {
    pub collection_id: InnerGenerationsCollectionId,
}

pub enum DatabaseCollectionGenerationsTask {
    Init(Arc<DatabaseInner>),
    NewCollection(NewCollectionGenerationsTask),
    DropCollection(DropCollectionGenerationsTask),
}
