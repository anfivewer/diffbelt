use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::response::{BaseResponse, BytesVecResponse, Response};
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;

use crate::collection::methods::put::CollectionPutOptions;

use crate::http::constants::PUT_REQUEST_MAX_BYTES;

use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;

use crate::http::data::encoded_generation_id::{
    EncodedGenerationIdFlatJsonData, EncodedOptionalGenerationIdFlatJsonData,
};
use crate::http::data::encoded_phantom_id::EncodedOptionalPhantomIdFlatJsonData;
use crate::http::data::key_value_update::KeyValueUpdateJsonData;
use crate::http::util::get_collection::get_collection;
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutRequestJsonData {
    #[serde(flatten)]
    item: KeyValueUpdateJsonData,

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
struct PutResponseJsonData {
    #[serde(flatten)]
    generation_id: EncodedGenerationIdFlatJsonData,
    was_put: Option<bool>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_id = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, PUT_REQUEST_MAX_BYTES).await?;
    let data: PutRequestJsonData = read_json(body)?;

    let collection = get_collection(&context, &collection_id).await?;

    let decoder = StringDecoder::from_default_encoding_string("encoding", data.encoding)?;

    let update = data.item.deserialize(&decoder)?;
    let if_not_present = update.if_not_present;

    let (generation_id, generation_id_encoding_type) =
        data.generation_id.decode_with_type(&decoder)?;

    let phantom_id = data.phantom_id.decode(&decoder)?;

    let options = CollectionPutOptions {
        update,
        generation_id,
        phantom_id,
    };

    let result = collection.put(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("put error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = PutResponseJsonData {
        generation_id: EncodedGenerationIdFlatJsonData::encode(
            result.generation_id.as_ref(),
            generation_id_encoding_type,
        ),
        was_put: if if_not_present {
            Some(result.was_put)
        } else {
            None
        },
    };

    let response = serde_json::to_vec(&response).or(Err(HttpError::PublicInternal500(
        "result serialization failed",
    )))?;

    Ok(Response::BytesVec(BytesVecResponse {
        base: BaseResponse {
            content_type: "application/json; charset=utf-8",
            ..Default::default()
        },
        bytes: response,
    }))
}

pub fn register_put_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/put$").unwrap(),
        id_only_group,
        handler,
    );
}
