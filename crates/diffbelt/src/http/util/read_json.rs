use crate::http::errors::HttpError;
use diffbelt_util::http::read_full_body::FullBody;
use serde::de::DeserializeOwned;

pub fn read_json<R: DeserializeOwned>(body: FullBody) -> Result<R, HttpError> {
    // TODO: report more information from error
    serde_json::from_reader(body).map_err(|err| HttpError::InvalidJson(err.to_string()))
}
