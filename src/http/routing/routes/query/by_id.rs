use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::collection::methods::query::ReadQueryCursorOptions;
use crate::context::Context;
use crate::http::constants::QUERY_START_REQUEST_MAX_BYTES;

use crate::http::data::query_response::QueryResponseJsonData;
use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::routing::routes::query::abort::abort_cursor;
use crate::http::routing::routes::query::next::read_cursor;
use crate::http::util::common_groups::{id_only_group, id_with_name_group, IdOnlyGroup, IdWithNameGroup};
use crate::http::util::get_collection::get_collection;

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
async fn handler(options: PatternRouteOptions<IdWithNameGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.id;
    let cursor_id = options.groups.name;

    let collection = get_collection(&context, &collection_name).await?;

    match request.method() {
        "GET" => read_cursor(request, collection, cursor_id).await,
        "DELETE" => abort_cursor(request, collection, cursor_id).await,
        _ => Err(HttpError::MethodNotAllowed),
    }
}

pub fn register_next_query_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/query/(?P<name>[^/]+)$").unwrap(),
        id_with_name_group,
        handler,
    );
}
