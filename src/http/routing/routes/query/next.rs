use std::sync::Arc;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;
use crate::collection::Collection;

use crate::collection::methods::query::ReadQueryCursorOptions;
use crate::context::Context;
use crate::http::constants::QUERY_START_REQUEST_MAX_BYTES;

use crate::http::data::query_response::QueryResponseJsonData;
use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::routing::response::Response;
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};

use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

pub async fn read_cursor(
    _request: impl Request,
    collection: Arc<Collection>,
    cursor_id: String,
) -> Result<Response, HttpError> {
    let options = ReadQueryCursorOptions { cursor_id };

    let result = collection.read_query_cursor(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("query/start error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = QueryResponseJsonData::from(result);
    create_ok_json_response(&response)
}
