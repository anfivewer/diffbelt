use crate::collection::methods::query::AbortQueryCursorOptions;
use crate::collection::Collection;
use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::Response;
use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use std::sync::Arc;

pub async fn abort_cursor(
    _request: impl Request,
    collection: Arc<Collection>,
    cursor_id: String,
) -> Result<Response, HttpError> {
    let result = collection
        .abort_query_cursor(AbortQueryCursorOptions { cursor_id })
        .await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("query/abort error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}
