use crate::collection::methods::list_readers::ListReadersOk;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::data::reader_record::ReaderRecordJsonData;

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
    // TODO: add filter
}

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
    let collection_id = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, READER_REQUEST_MAX_BYTES).await?;
    let _: RequestJsonData = read_json(body)?;

    let collection = get_collection(&context, &collection_id).await?;

    let result = collection.list_readers().await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/list error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let ListReadersOk { items } = result;

    let response = ResponseJsonData {
        items: ReaderRecordJsonData::encode_vec(items),
    };

    create_ok_json_response(&response)
}

pub fn register_list_readers_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/reader/list$").unwrap(),
        id_only_group,
        handler,
    );
}
