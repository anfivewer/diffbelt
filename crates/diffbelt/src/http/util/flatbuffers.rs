use diffbelt_protos::InvalidFlatbuffer;
use crate::http::errors::HttpError;

pub fn map_invalid_flatbuffer_error_to_http_error(err: InvalidFlatbuffer) -> HttpError {
    HttpError::InvalidFlatbuffers(format!("{err:?}"))
}