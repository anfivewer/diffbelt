use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::response::{BaseResponse, BytesVecResponse, Response};
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

use crate::collection::methods::put::CollectionPutOptions;
use crate::common::{
    IsByteArray, KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId,
    OwnedPhantomId,
};
use crate::http::constants::PUT_REQUEST_MAX_BYTES;

use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::util::json::serde::deserialize_strict_null;

use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutRequestJsonData {
    collection_id: String,

    key: String,
    key_encoding: Option<String>,
    if_not_present: Option<bool>,

    #[serde(deserialize_with = "deserialize_strict_null")]
    value: Option<String>,
    value_encoding: Option<String>,

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
struct PutResponseJsonData {
    generation_id: String,
    generation_id_encoding: Option<String>,
    was_put: Option<bool>,
}

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        let body = read_limited_body(request, PUT_REQUEST_MAX_BYTES).await?;
        let data: PutRequestJsonData = read_json(body)?;

        let collection_id = data.collection_id;

        let collection = context.database.get_collection(&collection_id).await;
        let Some(collection) = collection else { return Err(HttpError::Generic400("no such collection")); };

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

        let value = decoder.decode_opt_field_with_map(
            "value",
            data.value,
            "valueEncoding",
            data.value_encoding,
            |bytes| Ok(OwnedCollectionValue::new(&bytes)),
        )?;

        let (generation_id, generation_id_encoding_type) = decoder
            .decode_opt_field_with_map_and_type(
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

        let if_not_present = data.if_not_present.unwrap_or(false);

        let options = CollectionPutOptions {
            update: KeyValueUpdate {
                key,
                value,
                if_not_present,
            },
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

        let (generation_id, generation_id_encoding_type) = generation_id_encoding_type
            .serialize_with_priority(result.generation_id.get_byte_array());

        let response = PutResponseJsonData {
            generation_id,
            generation_id_encoding: generation_id_encoding_type.to_optional_string(),
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
    })
}

fn into_encoding_or_err(
    field_name: &str,
    encoding: Option<String>,
    default_encoding: StrSerializationType,
) -> Result<StrSerializationType, HttpError> {
    let result = match encoding {
        Some(encoding) => StrSerializationType::from_str(&encoding),
        None => {
            return Ok(default_encoding);
        }
    };

    result.or(Err(HttpError::GenericString400(format!(
        "invalid {}, allowed \"base64\" or default (\"utf8\")",
        field_name
    ))))
}

fn into_decoded_value(
    field_name: &str,
    value: String,
    encoding: StrSerializationType,
) -> Result<Box<[u8]>, HttpError> {
    encoding
        .deserialize(&value)
        .map_err(|_| HttpError::GenericString400(format!("invalid {}, check encoding", field_name)))
}

pub fn register_put_route(context: &mut Context) {
    context.routing.add_static_get_route("/put", handler);
}
