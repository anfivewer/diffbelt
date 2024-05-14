use crate::http::errors::HttpError;

pub fn no_such_collection_error() -> HttpError {
    HttpError::CustomJson400(r#"{"error":"noSuchCollection"}"#)
}
