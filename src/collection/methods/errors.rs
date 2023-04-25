use crate::collection::util::reader_value::OwnedReaderValue;
use crate::database::cursors::storage::CursorError;
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
