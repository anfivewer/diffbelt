use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::sync::Arc;

use crate::collection::methods::get_keys_around::CollectionGetKeysAroundOptions;
use crate::common::{OwnedCollectionKey, OwnedGenerationId, OwnedPhantomId};
use crate::context::Context;
use crate::http::constants::GET_KEYS_AROUND_REQUEST_MAX_BYTES;
use crate::http::data::bytes_generation_id::{
    flatbuffers_generation_id_to_owned_generation_id_opt,
    flatbuffers_phantom_id_to_owned_phantom_id_opt,
};
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_decode_opt, encoded_generation_id_data_encode,
    EncodedGenerationIdJsonData,
};
use crate::http::data::encoded_key::{EncodedKeyJsonData, EncodedKeyJsonDataTrait};
use crate::http::data::encoded_phantom_id::EncodedPhantomIdJsonDataTrait;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use diffbelt_protos::protos::handlers::{ApiHandlerRequestFlatType, GetKeysAroundApiHandler};
use diffbelt_types::common::phantom_id::EncodedPhantomIdJsonData;
use diffbelt_util_no_std::cast::u32_to_usize;

struct UnifiedRequestData<'a> {
    collection_name: &'a str,
    key: OwnedCollectionKey,
    require_key_existance: bool,
    limit: usize,
    generation_id: Option<OwnedGenerationId>,
    phantom_id: Option<OwnedPhantomId>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    key: EncodedKeyJsonData,

    require_key_existance: bool,
    limit: usize,

    generation_id: Option<EncodedGenerationIdJsonData>,
    phantom_id: Option<EncodedPhantomIdJsonData>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    generation_id: EncodedGenerationIdJsonData,

    left: Vec<EncodedKeyJsonData>,
    right: Vec<EncodedKeyJsonData>,

    has_more_on_the_left: bool,
    has_more_on_the_right: bool,

    found_key: bool,
}

async fn unified_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'_>,
) -> HttpHandlerResult {
    let UnifiedRequestData {
        collection_name,
        key,
        require_key_existance,
        limit,
        generation_id,
        phantom_id,
    } = data;

    let collection = context
        .database
        .get_collection(collection_name)
        .await
        .ok_or_else(|| HttpError::Generic400("no such collection"))?;

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
            eprintln!("get_keys_around error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    if request_context.is_flatbuffers() {
        todo!()
    } else {
        let response = ResponseJsonData {
            generation_id: encoded_generation_id_data_encode(
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
}

#[fn_box_pin_async]
async fn json_handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, GET_KEYS_AROUND_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let require_key_existance = data.require_key_existance;
    let limit = data.limit;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let key = data.key.decode(&decoder)?;
    let generation_id = encoded_generation_id_data_decode_opt(data.generation_id)?;
    let phantom_id = EncodedPhantomIdJsonData::decode_opt(data.phantom_id, &decoder)?;

    unified_handler(
        context,
        options.request_context,
        UnifiedRequestData {
            collection_name: &collection_name,
            key,
            require_key_existance,
            limit,
            generation_id,
            phantom_id,
        },
    )
    .await
}

pub fn register_get_keys_around_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/getKeysAround$").unwrap(),
        id_only_group,
        json_handler,
    );
}

pub async fn get_keys_around_flatbuffers_route(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, GetKeysAroundApiHandler>,
) -> HttpHandlerResult {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::Generic400("no collection name"))?;
    let key = request
        .key()
        .ok_or_else(|| HttpError::Generic400("no key"))?
        .bytes();
    let key = OwnedCollectionKey::from_boxed_slice(Box::from(key))
        .map_err(|_| HttpError::Generic400("invalid key"))?;
    let generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(request.generation_id())?;
    let phantom_id = flatbuffers_phantom_id_to_owned_phantom_id_opt(request.phantom_id())?;
    let require_key_existance = request.require_key_existance();
    let limit = u32_to_usize(request.limit());

    let data = UnifiedRequestData {
        collection_name,
        key,
        require_key_existance,
        limit,
        generation_id,
        phantom_id,
    };

    unified_handler(context, request_context, data).await
}
