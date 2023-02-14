use crate::collection::methods::get::CollectionGetOptions;
use crate::common::{IsByteArray, OwnedCollectionKey, OwnedGenerationId, OwnedPhantomId};
use crate::context::Context;
use crate::http::constants::GET_KEYS_AROUND_REQUEST_MAX_BYTES;
use crate::http::data::key_value::KeyValueJsonData;
use crate::http::errors::HttpError;
use crate::http::routing::response::{BaseResponse, BytesVecResponse, Response};
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use crate::collection::methods::get_keys_around::CollectionGetKeysAroundOptions;
use crate::http::data::encoded_key::EncodedKeyJsonData;
use crate::http::util::response::create_ok_json_response;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    collection_id: String,

    key: String,
    key_encoding: Option<String>,

    require_key_existance: bool,
    limit: usize,

    generation_id: Option<String>,
    generation_id_encoding: Option<String>,

    phantom_id: Option<String>,
    phantom_id_encoding: Option<String>,

    // Default encoding for all fields
    encoding: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    generation_id: String,
    generation_id_encoding: Option<String>,

    left: Vec<EncodedKeyJsonData>,
    right: Vec<EncodedKeyJsonData>,

    has_more_on_the_left: bool,
    has_more_on_the_right: bool,

    found_key: bool,
}

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
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

        let key = decoder.decode_field_with_map(
            "key",
            data.key,
            "keyEncoding",
            data.key_encoding,
            |bytes| {
                OwnedCollectionKey::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid key, length should be <= 16777215",
                )))
            },
        )?;

        let generation_id = decoder.decode_opt_field_with_map(
            "generationId",
            data.generation_id,
            "generationIdEncoding",
            data.generation_id_encoding,
            |bytes| {
                OwnedGenerationId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid generationId, length should be <= 255",
                )))
            },
        )?;

        let phantom_id = decoder.decode_opt_field_with_map(
            "phantomId",
            data.phantom_id,
            "phantomIdEncoding",
            data.phantom_id_encoding,
            |bytes| {
                if bytes.is_empty() {
                    return Err(HttpError::Generic400(
                        "invalid phantomId, it cannot be empty",
                    ));
                }

                OwnedPhantomId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid phantomId, length should be <= 255",
                )))
            },
        )?;

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

        let (generation_id, generation_id_encoding) = StrSerializationType::Utf8
            .serialize_with_priority(result.generation_id.get_byte_array());

        let response = ResponseJsonData {
            generation_id,
            generation_id_encoding: generation_id_encoding.to_optional_string(),
            left: EncodedKeyJsonData::encode_vec(result.left),
            right: EncodedKeyJsonData::encode_vec(result.right),
            has_more_on_the_left: result.has_more_on_the_left,
            has_more_on_the_right: result.has_more_on_the_right,
            found_key: true,
        };

        create_ok_json_response(&response)
    })
}

pub fn register_get_keys_around_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/getKeysAround", handler);
}
