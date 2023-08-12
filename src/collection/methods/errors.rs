use crate::collection::util::reader_value::OwnedReaderValue;
use crate::database::cursors::storage::CursorError;
use crate::messages::generations::{
    CommitManualGenerationError, LockManualGenerationIdError, StartManualGenerationIdError,
};
use tokio::sync::oneshot;

use crate::raw_db::RawDbError;

#[derive(Debug)]
pub enum CollectionMethodError {
    OutdatedGeneration,
    PutPhantomWithoutGenerationId,
    CannotPutInManualCollection,
    UnsupportedOperationForThisCollectionType,
    InvalidKey,
    ReaderAlreadyExists(OwnedReaderValue),
    InvalidUtf8,
    InvalidReaderValue,
    NoSuchCursor,
    NotImplementedYet,
    NoSuchReader,
    NoSuchCollection,

    RawDb(RawDbError),
    Channels,
    TaskJoin,
    CannotDeleteRawDbPath(std::io::Error),
    OneshotRecv(oneshot::error::RecvError),
    QueryCursor(CursorError),
}

impl From<RawDbError> for CollectionMethodError {
    fn from(err: RawDbError) -> Self {
        CollectionMethodError::RawDb(err)
    }
}

impl From<oneshot::error::RecvError> for CollectionMethodError {
    fn from(value: oneshot::error::RecvError) -> Self {
        CollectionMethodError::OneshotRecv(value)
    }
}

impl From<LockManualGenerationIdError> for CollectionMethodError {
    fn from(value: LockManualGenerationIdError) -> Self {
        match value {
            LockManualGenerationIdError::GenerationIdMismatch => {
                CollectionMethodError::OutdatedGeneration
            }
            LockManualGenerationIdError::PutPhantomWithoutGenerationId => {
                CollectionMethodError::PutPhantomWithoutGenerationId
            }
            LockManualGenerationIdError::NoSuchCollection => {
                CollectionMethodError::NoSuchCollection
            }
        }
    }
}

impl From<StartManualGenerationIdError> for CollectionMethodError {
    fn from(value: StartManualGenerationIdError) -> Self {
        match value {
            StartManualGenerationIdError::OutdatedGeneration => {
                CollectionMethodError::OutdatedGeneration
            }
            StartManualGenerationIdError::RawDb(err) => CollectionMethodError::RawDb(err),
            StartManualGenerationIdError::NoSuchCollection => {
                CollectionMethodError::NoSuchCollection
            }
        }
    }
}

impl From<CommitManualGenerationError> for CollectionMethodError {
    fn from(value: CommitManualGenerationError) -> Self {
        match value {
            CommitManualGenerationError::OutdatedGeneration => {
                CollectionMethodError::OutdatedGeneration
            }
            CommitManualGenerationError::RawDb(err) => CollectionMethodError::RawDb(err),
            CommitManualGenerationError::NoSuchCollection => {
                CollectionMethodError::NoSuchCollection
            }
        }
    }
}
