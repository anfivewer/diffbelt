use std::fmt::Debug;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntoBytesError<T: Debug> {
    #[error("{0:?}")]
    UnknownEncoding(T),
    #[error("{0:?}")]
    Base64(T),
}
