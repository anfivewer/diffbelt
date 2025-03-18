use std::sync::Arc;

use crate::collection::methods::diff::AbortDiffCursorOptions;
use crate::collection::Collection;
use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::HttpResponse;
use crate::http::util::response::create_ok_no_error_json_response;

pub async fn abort_cursor(
    _request: impl Request,
    collection: Arc<Collection>,
    cursor_id: Box<str>,
) -> Result<HttpResponse, HttpError> {
    let result = collection
        .abort_diff_cursor(AbortDiffCursorOptions { cursor_id })
        .await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("diff/abort error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}
