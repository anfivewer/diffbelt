use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::context::Context;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::routes::diff::abort::abort_cursor;
use crate::http::routing::routes::diff::next::read_cursor;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::common_groups::{id_with_name_group, IdWithNameGroup};
use crate::http::util::get_collection::get_collection;

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

pub fn register_next_diff_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/diff/(?P<name>[^/]+)$").unwrap(),
        id_with_name_group,
        handler,
    );
}
