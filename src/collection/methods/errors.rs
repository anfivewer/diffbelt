use crate::collection::util::reader_value::OwnedReaderValue;

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
}

impl From<RawDbError> for CollectionMethodError {
    fn from(err: RawDbError) -> Self {
        CollectionMethodError::RawDb(err)
    }
}
