use crate::collection::methods::get_keys_around::CollectionGetKeysAroundOptions;

use crate::context::Context;
use crate::http::constants::GET_KEYS_AROUND_REQUEST_MAX_BYTES;
use crate::http::data::encoded_key::{EncodedKeyFlatJsonData, EncodedKeyJsonData};

use crate::http::errors::HttpError;

use crate::http::data::encoded_generation_id::{
    EncodedGenerationIdFlatJsonData, EncodedOptionalGenerationIdFlatJsonData,
};
use crate::http::data::encoded_phantom_id::EncodedOptionalPhantomIdFlatJsonData;
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    collection_id: String,

    #[serde(flatten)]
    key: EncodedKeyFlatJsonData,

    require_key_existance: bool,
    limit: usize,

    #[serde(flatten)]
    generation_id: EncodedOptionalGenerationIdFlatJsonData,

    #[serde(flatten)]
    phantom_id: EncodedOptionalPhantomIdFlatJsonData,

    // Default encoding for all fields
    encoding: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    #[serde(flatten)]
    generation_id: EncodedGenerationIdFlatJsonData,

    left: Vec<EncodedKeyJsonData>,
    right: Vec<EncodedKeyJsonData>,

    has_more_on_the_left: bool,
    has_more_on_the_right: bool,

    found_key: bool,
}

#[fn_box_pin_async]
async fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    let context = options.context;
    let request = options.request;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, GET_KEYS_AROUND_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let collection_id = data.collection_id;

    let collection = context.database.get_collection(&collection_id).await;
    let Some(collection) = collection else { return Err(HttpError::Generic400("no such collection")); };

    let require_key_existance = data.require_key_existance;
    let limit = data.limit;

    let decoder = StringDecoder::from_default_encoding_string("encoding", data.encoding)?;

    let key = data.key.decode(&decoder)?;
    let generation_id = data.generation_id.decode(&decoder)?;
    let phantom_id = data.phantom_id.decode(&decoder)?;

    let options = CollectionGetKeysAroundOptions {
        key,
        generation_id,
        phantom_id,
        require_key_existance,
        limit,
    };

    let result = collection.get_keys_around(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("get error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = ResponseJsonData {
        generation_id: EncodedGenerationIdFlatJsonData::encode(
            result.generation_id.as_ref(),
            StrSerializationType::Utf8,
        ),
        left: EncodedKeyJsonData::encode_vec(result.left),
        right: EncodedKeyJsonData::encode_vec(result.right),
        has_more_on_the_left: result.has_more_on_the_left,
        has_more_on_the_right: result.has_more_on_the_right,
        found_key: true,
    };

    create_ok_json_response(&response)
}

pub fn register_get_keys_around_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/getKeysAround", handler);
}
