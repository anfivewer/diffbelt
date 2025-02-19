use diffbelt_aligned_bytes::OwnedAlignedBytes;
use diffbelt_util::http::error::BodyReadError;
use diffbelt_util::http::read_full_body::FullBody;

use crate::http::errors::HttpError;
use crate::http::request::Request;

pub async fn read_limited_body<Req: Request>(
    request: Req,
    max_bytes: usize,
) -> Result<FullBody, HttpError> {
    request
        .into_full_body_as_read(max_bytes)
        .await
        .map_err(|err| body_read_error_to_http_error(err, max_bytes))
}

pub async fn read_limited_aligned_bytes<Req: Request, const ALIGN: usize>(
    request: Req,
    max_bytes: usize,
) -> Result<OwnedAlignedBytes<ALIGN>, HttpError> {
    request
        .into_aligned_bytes(max_bytes)
        .await
        .map_err(|err| body_read_error_to_http_error(err, max_bytes))
}

fn body_read_error_to_http_error(err: BodyReadError, max_bytes: usize) -> HttpError {
    match err {
        BodyReadError::IO => HttpError::Generic400("io"),
        BodyReadError::SizeLimit => HttpError::TooBigPayload(max_bytes),
        BodyReadError::Align(_) => HttpError::PublicInternal500("alignError"),
    }
}
