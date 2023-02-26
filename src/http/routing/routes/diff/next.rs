use crate::collection::methods::diff::ReadDiffCursorOptions;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::context::Context;
use crate::http::constants::QUERY_START_REQUEST_MAX_BYTES;
use crate::http::data::diff_response::DiffResponseJsonData;

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::get_collection::get_collection;
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    cursor_id: String,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, QUERY_START_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let collection = get_collection(&context, &collection_name).await?;

    let cursor_id = data.cursor_id;

    let options = ReadDiffCursorOptions { cursor_id };

    let result = collection.read_diff_cursor(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("diff/next error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = DiffResponseJsonData::from(result);
    create_ok_json_response(&response)
}

pub fn register_next_diff_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/diff/next$").unwrap(),
        id_only_group,
        handler,
    );
}
