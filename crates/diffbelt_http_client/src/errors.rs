use diffbelt_protos::error::FlatbufferError;
use diffbelt_protos::InvalidFlatbuffer;
use diffbelt_util::http::error::BodyReadError;
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
    #[error("{0:?}")]
    InvalidFlatbuffer(InvalidFlatbuffer),
    #[error("{0:?}")]
    Flatbuffer(FlatbufferError),
}

impl From<hyper::Error> for DiffbeltClientError {
    fn from(value: hyper::Error) -> Self {
        Self::Hyper(value)
    }
}

impl From<BodyReadError> for DiffbeltClientError {
    fn from(value: BodyReadError) -> Self {
        Self::BodyRead(value)
    }
}

impl From<InvalidFlatbuffer> for DiffbeltClientError {
    fn from(value: InvalidFlatbuffer) -> Self {
        Self::InvalidFlatbuffer(value)
    }
}

impl From<FlatbufferError> for DiffbeltClientError {
    fn from(value: FlatbufferError) -> Self {
        Self::Flatbuffer(value)
    }
}
