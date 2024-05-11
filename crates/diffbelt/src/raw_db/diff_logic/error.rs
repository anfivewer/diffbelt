use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffLogicError {
    #[error("InvalidRecordKey")]
    InvalidRecordKey,
    #[error("ChangedKeyNotFound")]
    ChangedKeyNotFound,
    #[error("NoFirstAndLastKeys")]
    NoFirstAndLastKeys,
    #[error("ChangeNotFound")]
    ChangeNotFound,
    #[error("AlreadyFinished")]
    AlreadyFinished,
    #[error("{0}")]
    Unspecified(String),
}
