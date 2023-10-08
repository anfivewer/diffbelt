use diffbelt_util::http::read_full_body::BodyReadError;

#[derive(Debug)]
pub enum DiffbeltClientError {
    Hyper(hyper::Error),
    BodyRead(BodyReadError),
    Not200Unknown,
    Not200(String),
    JsonParsing,
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
