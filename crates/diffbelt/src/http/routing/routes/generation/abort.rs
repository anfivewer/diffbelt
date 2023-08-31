use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::collection::methods::abort_generation::AbortGenerationOptions;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_into_generation_id, EncodedGenerationIdJsonData,
};

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::get_collection::get_collection;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    generation_id: EncodedGenerationIdJsonData,
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

    let RequestJsonData { generation_id } = data;

    let generation_id = encoded_generation_id_data_into_generation_id(generation_id)?;

    let collection = get_collection(&context, &collection_name).await?;

    let options = AbortGenerationOptions { generation_id };

    let result = collection.abort_generation(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("generation/abort error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}

pub fn register_abort_generation_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/generation/abort$").unwrap(),
        id_only_group,
        handler,
    );
}
