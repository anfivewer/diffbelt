use crate::collection::methods::delete_reader::DeleteReaderOptions;
use std::sync::Arc;

use crate::collection::Collection;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::Response;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::get_collection::get_collection;
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

pub async fn delete_reader(
    _request: impl Request,
    collection: Arc<Collection>,
    reader_name: String,
) -> Result<Response, HttpError> {
    let options = DeleteReaderOptions { reader_name };

    let result = collection.delete_reader(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/delete error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}
