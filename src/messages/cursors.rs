use crate::database::cursors::collection::InnerCursorsCollectionId;
use crate::database::cursors::query::{
    AddQueryCursorContinuationData, AddQueryCursorData, InnerQueryCursorId, QueryCursorError,
    QueryCursorPublicId, QueryCursorRef,
};
use tokio::sync::oneshot;

pub struct NewCollectionTask {
    pub sender: oneshot::Sender<InnerCursorsCollectionId>,
}

pub struct DropCollectionTask {
    pub collection_id: InnerCursorsCollectionId,
}

pub struct AddQueryCursorTask {
    pub collection_id: InnerCursorsCollectionId,
    pub data: AddQueryCursorData,
    pub sender: oneshot::Sender<Result<QueryCursorPublicId, QueryCursorError>>,
}

pub struct GetQueryCursorByPublicIdTask {
    pub collection_id: InnerCursorsCollectionId,
    pub public_id: QueryCursorPublicId,
    pub sender: oneshot::Sender<Option<(InnerQueryCursorId, QueryCursorRef)>>,
}

pub struct AddQueryCursorContinuationTask {
    pub collection_id: InnerCursorsCollectionId,
    pub inner_id: InnerQueryCursorId,
    pub is_current: bool,
    pub data: AddQueryCursorContinuationData,
    pub sender: oneshot::Sender<Result<QueryCursorPublicId, QueryCursorError>>,
}

pub struct FinishQueryCursorTask {
    pub collection_id: InnerCursorsCollectionId,
    pub inner_id: InnerQueryCursorId,
    pub is_current: bool,
    pub sender: oneshot::Sender<Result<QueryCursorPublicId, QueryCursorError>>,
}

pub struct FullyFinishQueryCursorTask {
    pub collection_id: InnerCursorsCollectionId,
    pub inner_id: InnerQueryCursorId,
    pub sender: oneshot::Sender<Result<(), QueryCursorError>>,
}

pub struct AbortQueryCursorTask {
    pub collection_id: InnerCursorsCollectionId,
    pub public_id: QueryCursorPublicId,
    pub sender: oneshot::Sender<Result<(), QueryCursorError>>,
}

#[cfg(test)]
pub struct GetCollectionQueryCursorsCountTask {
    pub collection_id: InnerCursorsCollectionId,
    pub sender: oneshot::Sender<Result<usize, QueryCursorError>>,
}

pub enum DatabaseCollectionCursorsTask {
    NewCollection(NewCollectionTask),
    DropCollection(DropCollectionTask),
    AddQueryCursor(AddQueryCursorTask),
    GetQueryCursorByPublicId(GetQueryCursorByPublicIdTask),
    AddQueryCursorContinuation(AddQueryCursorContinuationTask),
    FinishQueryCursor(FinishQueryCursorTask),
    FullyFinishQueryCursor(FullyFinishQueryCursorTask),
    AbortQueryCursor(AbortQueryCursorTask),

    #[cfg(test)]
    GetCollectionQueryCursorsCount(GetCollectionQueryCursorsCountTask),
}
