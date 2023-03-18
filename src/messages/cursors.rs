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
    pub inner_id: InnerCursorsCollectionId,
}

pub struct AddQueryCursorTask {
    pub data: AddQueryCursorData,
    pub sender: oneshot::Sender<QueryCursorPublicId>,
}

pub struct GetQueryCursorByPublicIdTask {
    pub public_id: QueryCursorPublicId,
    pub sender: oneshot::Sender<Option<(InnerQueryCursorId, QueryCursorRef)>>,
}

pub struct AddQueryCursorContinuationTask {
    pub inner_id: InnerQueryCursorId,
    pub data: AddQueryCursorContinuationData,
    pub sender: oneshot::Sender<Result<QueryCursorPublicId, QueryCursorError>>,
}

pub struct FinishQueryCursorTask {
    pub inner_id: InnerQueryCursorId,
    pub sender: oneshot::Sender<Result<QueryCursorPublicId, QueryCursorError>>,
}

pub struct FullyFinishQueryCursorTask {
    pub inner_id: InnerQueryCursorId,
    pub sender: oneshot::Sender<Result<(), QueryCursorError>>,
}

pub enum DatabaseCollectionCursorsTask {
    NewCollection(NewCollectionTask),
    DropCollection(DropCollectionTask),
    AddQueryCursor(AddQueryCursorTask),
    GetQueryCursorByPublicId(GetQueryCursorByPublicIdTask),
    AddQueryCursorContinuation(AddQueryCursorContinuationTask),
    FinishQueryCursor(FinishQueryCursorTask),
    FullyFinishQueryCursorTask(FullyFinishQueryCursorTask),
}
