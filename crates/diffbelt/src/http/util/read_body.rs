use crate::http::errors::HttpError;
use crate::http::request::Request;
use diffbelt_util::http::read_full_body::{BodyReadError, FullBody};

pub async fn read_limited_body<Req: Request>(
    request: Req,
    max_bytes: usize,
) -> Result<FullBody, HttpError> {
    request
        .into_full_body_as_read(max_bytes)
        .await
        .map_err(|err| match err {
            BodyReadError::IO => HttpError::Generic400("io"),
            BodyReadError::SizeLimit => HttpError::TooBigPayload(max_bytes),
        })
}
