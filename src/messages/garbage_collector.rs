use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::common::collection::CollectionName;
use crate::common::OwnedGenerationId;
use crate::database::DatabaseInner;
use crate::util::auto_sender_on_drop::AutoSenderOnDrop;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

pub enum GarbageCollectorCommonError {
    SuchCollectionAlreadyExists,
}

pub struct NewCollectionTaskResponse {
    pub id: usize,
    pub drop_handle: AutoSenderOnDrop<()>,
}

pub struct GarbageCollectorNewCollectionTask {
    pub collection_name: CollectionName,
    pub raw_db: CollectionRawDb,
    pub is_deleted: Arc<RwLock<bool>>,
    pub sender: oneshot::Sender<Result<NewCollectionTaskResponse, GarbageCollectorCommonError>>,
}

pub struct GarbageCollectorDropCollectionTask {
    pub collection_name: CollectionName,
    pub id: usize,
    pub sender: Option<oneshot::Sender<()>>,
}

pub struct CleanupGenerationsLessThanTask {
    pub collection_name: CollectionName,
    pub generation_id_less_than: OwnedGenerationId,
}

pub enum DatabaseGarbageCollectorTask {
    Init(Arc<DatabaseInner>),
    NewCollection(GarbageCollectorNewCollectionTask),
    DropCollection(GarbageCollectorDropCollectionTask),
    CleanupGenerationsLessThan(CleanupGenerationsLessThanTask),
}
