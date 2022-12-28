use crate::http::errors::HttpError;
use crate::http::request::FullBody;
use serde::de::DeserializeOwned;

pub fn read_json<R: DeserializeOwned>(body: FullBody) -> Result<R, HttpError> {
    // TODO: report more information from error
    serde_json::from_reader(body).or(Err(HttpError::InvalidJson))
}
