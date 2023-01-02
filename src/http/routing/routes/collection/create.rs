use crate::common::constants::MAX_COLLECTION_ID_LENGTH;
use crate::common::OwnedGenerationId;
use crate::context::Context;
use crate::database::create_collection::{CreateCollectionError, CreateCollectionOptions};
use crate::http::constants::CREATE_COLLECTION_REQUEST_MAX_BYTES;
use crate::http::errors::HttpError;

use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCollectionRequestJsonData {
    collection_id: String,
    is_manual: bool,
    // Only for manual collections
    initial_generation_id: Option<String>,
    initial_generation_id_encoding: Option<String>,
    encoding: Option<String>,
}

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        let body = read_limited_body(request, CREATE_COLLECTION_REQUEST_MAX_BYTES).await?;
        let data: CreateCollectionRequestJsonData = read_json(body)?;

        let collection_id = data.collection_id;
        let is_manual = data.is_manual;

        if collection_id.len() > MAX_COLLECTION_ID_LENGTH {
            return Err(HttpError::Generic400("collectionId cannot be > 512"));
        }

        let decoder = StringDecoder::from_default_encoding_string("encoding", data.encoding)?;

        let initial_generation_id = decoder.decode_opt_field_with_map(
            "initialGenerationId",
            data.initial_generation_id,
            "initialGenerationIdEncoding",
            data.initial_generation_id_encoding,
            |bytes| {
                OwnedGenerationId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid generationId, length should be <= 255",
                )))
            },
        )?;

        if initial_generation_id.is_some() != is_manual {
            return Err(HttpError::Generic400(
                "initialGenerationId should be present if isManual and be absent if !isManual",
            ));
        }

        let result = context
            .database
            .create_collection(&collection_id, CreateCollectionOptions { is_manual })
            .await;

        if let Err(err) = result {
            return match err {
                CreateCollectionError::AlreadyExist => Err(HttpError::Generic400(
                    "collection with such id already exists",
                )),
                _ => {
                    eprintln!("create collection error {:?}", err);
                    Err(HttpError::Unspecified)
                }
            };
        }

        create_ok_no_error_json_response()
    })
}

pub fn register_create_collection_route(context: &mut Context) {
    context
        .routing
        .add_static_get_route("/collection/create", handler);
}
