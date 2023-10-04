use crate::collection::methods::get::CollectionGetOptions;

use crate::context::Context;
use crate::http::constants::GET_REQUEST_MAX_BYTES;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_decode_opt, encoded_generation_id_data_encode,
    EncodedGenerationIdJsonData,
};
use crate::http::data::encoded_key::{EncodedKeyJsonData, EncodedKeyJsonDataTrait};
use crate::http::data::encoded_phantom_id::EncodedPhantomIdJsonData;
use crate::http::data::key_value::KeyValueJsonData;
use crate::http::errors::HttpError;

use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetRequestJsonData {
    key: EncodedKeyJsonData,
    generation_id: Option<EncodedGenerationIdJsonData>,
    phantom_id: Option<EncodedPhantomIdJsonData>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetResponseJsonData {
    generation_id: EncodedGenerationIdJsonData,

    #[serialize_always]
    item: Option<KeyValueJsonData>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, GET_REQUEST_MAX_BYTES).await?;
    let data: GetRequestJsonData = read_json(body)?;

    let collection = context.database.get_collection(&collection_name).await;
    let Some(collection) = collection else {
        return Err(HttpError::Generic400("no such collection"));
    };

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let key = EncodedKeyJsonData::decode(data.key, &decoder)?;
    let generation_id = encoded_generation_id_data_decode_opt(data.generation_id)?;
    let phantom_id = EncodedPhantomIdJsonData::decode_opt(data.phantom_id, &decoder)?;

    let options = CollectionGetOptions {
        key,
        generation_id,
        phantom_id,
    };

    let result = collection.get(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("get error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = GetResponseJsonData {
        generation_id: encoded_generation_id_data_encode(
            result.generation_id.as_ref(),
            StrSerializationType::Utf8,
        ),
        item: result.item.map(|item| item.into()),
    };

    create_ok_json_response(&response)
}

pub fn register_get_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/get$").unwrap(),
        id_only_group,
        handler,
    );
}
