use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::collection::methods::query::QueryOptions;
use crate::context::Context;
use crate::http::constants::QUERY_START_REQUEST_MAX_BYTES;
use crate::http::data::encoded_generation_id::EncodedOptionalGenerationIdFlatJsonData;
use crate::http::data::encoded_phantom_id::EncodedOptionalPhantomIdFlatJsonData;

use crate::http::data::query_response::QueryResponseJsonData;
use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    #[serde(flatten)]
    generation_id: EncodedOptionalGenerationIdFlatJsonData,
    #[serde(flatten)]
    phantom_id: EncodedOptionalPhantomIdFlatJsonData,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_id = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, QUERY_START_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let generation_id = data.generation_id.decode(&decoder)?;
    let phantom_id = data.phantom_id.decode(&decoder)?;

    let collection = get_collection(&context, &collection_id).await?;

    let options = QueryOptions {
        generation_id,
        phantom_id,
    };

    let result = collection.query(options).await;

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

pub fn register_start_query_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/query/start$").unwrap(),
        id_only_group,
        handler,
    );
}
