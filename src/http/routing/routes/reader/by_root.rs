use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::context::Context;

use crate::http::data::reader_record::ReaderRecordJsonData;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::routes::reader::create::create_reader;
use crate::http::routing::routes::reader::list::list_readers;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::get_collection::get_collection;

use crate::http::validation::ContentTypeValidation;

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
