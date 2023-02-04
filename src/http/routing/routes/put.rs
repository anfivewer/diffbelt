use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::response::{BaseResponse, BytesVecResponse, Response};
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

use crate::collection::methods::put::CollectionPutOptions;
use crate::common::{IsByteArray, OwnedGenerationId, OwnedPhantomId};
use crate::http::constants::PUT_REQUEST_MAX_BYTES;

use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;

use crate::http::data::key_value_update::KeyValueUpdateJsonData;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutRequestJsonData {
    collection_id: String,

    #[serde(flatten)]
    item: KeyValueUpdateJsonData,

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

        let update = data.item.deserialize(&decoder)?;
        let if_not_present = update.if_not_present;

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

pub fn register_put_route(context: &mut Context) {
    context.routing.add_static_post_route("/put", handler);
}
