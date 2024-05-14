use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffLogicError {
    #[error("InvalidRecordKey")]
    InvalidRecordKey,
    #[error("ChangedKeyNotFound({0})")]
    ChangedKeyNotFound(&'static str),
    #[error("NoFirstAndLastKeys")]
    NoFirstAndLastKeys,
    #[error("ChangeNotFound")]
    ChangeNotFound,
    #[error("AlreadyFinished")]
    AlreadyFinished,
    #[error("AlreadyErrored")]
    AlreadyErrored,
    #[error("{0}")]
    Unspecified(String),
}
