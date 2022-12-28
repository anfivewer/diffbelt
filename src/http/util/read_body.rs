use crate::http::errors::HttpError;
use crate::http::request::{FullBody, Request, RequestReadError};

pub async fn read_limited_body<Req: Request>(
    request: Req,
    max_bytes: usize,
) -> Result<FullBody, HttpError> {
    request
        .into_full_body_as_read(max_bytes)
        .await
        .map_err(|err| match err {
            RequestReadError::IO => HttpError::Generic400("io"),
            RequestReadError::SizeLimit => HttpError::TooBigPayload(max_bytes),
        })
}
