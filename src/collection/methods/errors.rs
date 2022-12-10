use crate::raw_db::RawDbError;

#[derive(Debug)]
pub enum CollectionMethodError {
    OutdatedGeneration,
    PutPhantomWithoutGenerationId,
    CannotPutInManualCollection,
    InvalidKey,

    RawDb(RawDbError),
    Channels,
}

impl From<RawDbError> for CollectionMethodError {
    fn from(err: RawDbError) -> Self {
        CollectionMethodError::RawDb(err)
    }
}
