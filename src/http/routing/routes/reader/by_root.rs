use crate::collection::methods::list_readers::ListReadersOk;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::data::reader_record::ReaderRecordJsonData;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::routing::routes::reader::create::create_reader;
use crate::http::routing::routes::reader::list::list_readers;

use crate::http::util::get_collection::get_collection;
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    items: Vec<ReaderRecordJsonData>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_utf8_json_by_default()?;

    let collection = get_collection(&context, &collection_name).await?;

    match request.method() {
        "GET" => list_readers(request, collection).await,
        "POST" => create_reader(request, collection).await,
        _ => Err(HttpError::MethodNotAllowed),
    }
}

pub fn register_readers_root_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/readers/$").unwrap(),
        id_only_group,
        handler,
    );
}
