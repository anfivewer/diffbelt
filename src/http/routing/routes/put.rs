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
use crate::http::request::{Request, RequestReadError};
use crate::util::json::serde::deserialize_strict_null;
use crate::util::option::lift_result_from_option;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutRequestJsonData {
    collection_id: String,

    key: String,
    #[serde(default)]
    key_encoding: Option<String>,
    #[serde(default)]
    if_not_present: Option<bool>,

    #[serde(deserialize_with = "deserialize_strict_null")]
    value: Option<String>,
    #[serde(default)]
    value_encoding: Option<String>,

    #[serde(default)]
    generation_id: Option<String>,
    #[serde(default)]
    generation_id_encoding: Option<String>,

    #[serde(default)]
    phantom_id: Option<String>,
    #[serde(default)]
    phantom_id_encoding: Option<String>,

    // Default encoding for all fields
    #[serde(default)]
    encoding: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PutResponseJsonData {
    generation_id: String,
    generation_id_encoding: Option<String>,
    was_put: Option<bool>,
}

const MAX_SIZE: usize = 4 * 1024 * 1024;

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        let body = request
            .into_full_body_as_read(MAX_SIZE)
            .await
            .map_err(|err| match err {
                RequestReadError::IO => HttpError::Generic400("io"),
                RequestReadError::SizeLimit => HttpError::TooBigPayload(MAX_SIZE),
            })?;

        let data: PutRequestJsonData =
            serde_json::from_reader(body).or(Err(HttpError::InvalidJson))?;

        let collection_id = data.collection_id;

        let collection = context.database.get_collection(&collection_id).await;
        let Some(collection) = collection else { return Err(HttpError::Generic400("no such collection")); };

        let default_encoding_type =
            into_encoding_or_err("encoding", data.encoding, StrSerializationType::Utf8)?;
        let key_encoding_type =
            into_encoding_or_err("keyEncoding", data.key_encoding, default_encoding_type)?;
        let value_encoding_type =
            into_encoding_or_err("valueEncoding", data.value_encoding, default_encoding_type)?;
        let generation_id_encoding_type = into_encoding_or_err(
            "generationIdEncoding",
            data.generation_id_encoding,
            default_encoding_type,
        )?;
        let phantom_id_encoding_type = into_encoding_or_err(
            "phantomIdEncoding",
            data.phantom_id_encoding,
            default_encoding_type,
        )?;

        let if_not_present = data.if_not_present.unwrap_or(false);

        let key = into_decoded_value("key", data.key, key_encoding_type)?;
        let value = lift_result_from_option(
            data.value
                .map(|value| into_decoded_value("value", value, value_encoding_type)),
        )?;

        let generation_id =
            lift_result_from_option(data.generation_id.map(|value| {
                into_decoded_value("generationId", value, generation_id_encoding_type)
            }))?;
        let generation_id = lift_result_from_option(
            generation_id.map(|id| OwnedGenerationId::from_boxed_slice(id)),
        )
        .or(Err(HttpError::Generic400(
            "invalid generationId, length should be <= 255",
        )))?;

        let phantom_id = lift_result_from_option(data.phantom_id.map(|value| {
            if value.is_empty() {
                return Err(HttpError::Generic400(
                    "invalid phantomId, it cannot be empty",
                ));
            }

            into_decoded_value("phantomId", value, phantom_id_encoding_type)
        }))?;
        let phantom_id = lift_result_from_option(
            phantom_id.map(|id| OwnedPhantomId::from_boxed_slice(id)),
        )
        .or(Err(HttpError::Generic400(
            "invalid phantomId, length should be <= 255",
        )))?;

        let options = CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey::from_boxed_slice(key).or(Err(HttpError::Generic400(
                    "invalid key, length should be <= 16777215",
                )))?,
                value: value.map(|value| OwnedCollectionValue::new(&value)),
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
