use crate::raw_db::RawDbError;

pub enum CollectionMethodError {
    OutdatedGeneration,
    PutPhantomWithoutGenerationId,
    CannotPutInManualCollection,
    NextGenerationIsNotStarted,
    InvalidKey,

    RawDb(RawDbError),
    Channels,
}

impl From<RawDbError> for CollectionMethodError {
    fn from(err: RawDbError) -> Self {
        CollectionMethodError::RawDb(err)
    }
}
