use diffbelt_util::http::read_full_body::BodyReadError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiffbeltClientError {
    #[error(transparent)]
    Hyper(hyper::Error),
    #[error("{0:?}")]
    BodyRead(BodyReadError),
    #[error("Not200Unknown")]
    Not200Unknown,
    #[error("Not200: {0}")]
    Not200(String),
    #[error("JsonParsing")]
    JsonParsing,
    #[error(transparent)]
    JsonSerialize(serde_json::Error),
}

impl From<hyper::Error> for DiffbeltClientError {
    fn from(value: hyper::Error) -> Self {
        DiffbeltClientError::Hyper(value)
    }
}

impl From<BodyReadError> for DiffbeltClientError {
    fn from(value: BodyReadError) -> Self {
        DiffbeltClientError::BodyRead(value)
    }
}
