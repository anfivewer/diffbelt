use crate::database::cursors::collection::InnerCursorsCollectionId;
use crate::database::cursors::diff::DiffCursorType;
use crate::database::cursors::query::QueryCursorType;
use crate::database::cursors::storage::{
    CursorError, CursorPublicId, CursorRef, CursorType, InnerCursorId,
};
use std::marker::PhantomData;
use tokio::sync::oneshot;

pub struct NewCollectionTask {
    pub sender: oneshot::Sender<InnerCursorsCollectionId>,
}

pub struct DropCollectionTask {
    pub collection_id: InnerCursorsCollectionId,
}

pub struct AddCursorTask<T: CursorType> {
    pub collection_id: InnerCursorsCollectionId,
    pub data: T::AddData,
    pub sender: oneshot::Sender<Result<CursorPublicId, CursorError>>,
}

pub struct GetCursorByPublicIdTask<T: CursorType> {
    pub collection_id: InnerCursorsCollectionId,
    pub public_id: CursorPublicId,
    pub sender: oneshot::Sender<Option<(InnerCursorId<T>, CursorRef<T>)>>,
}

pub struct AddCursorContinuationTask<T: CursorType> {
    pub collection_id: InnerCursorsCollectionId,
    pub inner_id: InnerCursorId<T>,
    pub is_current: bool,
    pub data: T::AddContinuationData,
    pub sender: oneshot::Sender<Result<CursorPublicId, CursorError>>,
}

pub struct FinishCursorTask<T: CursorType> {
    pub collection_id: InnerCursorsCollectionId,
    pub inner_id: InnerCursorId<T>,
    pub is_current: bool,
    pub sender: oneshot::Sender<Result<CursorPublicId, CursorError>>,
}

pub struct FullyFinishCursorTask<T: CursorType> {
    pub collection_id: InnerCursorsCollectionId,
    pub inner_id: InnerCursorId<T>,
    pub sender: oneshot::Sender<Result<(), CursorError>>,
}

pub struct AbortCursorTask<T: CursorType> {
    pub cursor_type: PhantomData<T>,
    pub collection_id: InnerCursorsCollectionId,
    pub public_id: CursorPublicId,
    pub sender: oneshot::Sender<Result<(), CursorError>>,
}

#[cfg(test)]
pub struct GetCollectionCursorsCountTask<T: CursorType> {
    pub cursor_type: PhantomData<T>,
    pub collection_id: InnerCursorsCollectionId,
    pub sender: oneshot::Sender<Result<usize, CursorError>>,
}

pub enum DatabaseCollectionSpecificCursorsTask<T: CursorType> {
    AddQueryCursor(AddCursorTask<T>),
    GetQueryCursorByPublicId(GetCursorByPublicIdTask<T>),
    AddQueryCursorContinuation(AddCursorContinuationTask<T>),
    FinishQueryCursor(FinishCursorTask<T>),
    FullyFinishQueryCursor(FullyFinishCursorTask<T>),
    AbortQueryCursor(AbortCursorTask<T>),

    #[cfg(test)]
    GetCollectionQueryCursorsCount(GetCollectionCursorsCountTask<T>),
}

pub enum DatabaseCollectionCursorsTask {
    NewCollection(NewCollectionTask),
    DropCollection(DropCollectionTask),
    Query(DatabaseCollectionSpecificCursorsTask<QueryCursorType>),
    Diff(DatabaseCollectionSpecificCursorsTask<DiffCursorType>),
}
