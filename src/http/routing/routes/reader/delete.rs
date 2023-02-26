use crate::collection::methods::delete_reader::DeleteReaderOptions;

use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::get_collection::get_collection;
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    reader_name: String,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, READER_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let RequestJsonData { reader_name } = data;

    let collection = get_collection(&context, &collection_name).await?;

    let options = DeleteReaderOptions {
        reader_name: reader_name,
    };

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

pub fn register_delete_reader_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/reader/delete$").unwrap(),
        id_only_group,
        handler,
    );
}
