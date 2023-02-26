use crate::common::constants::MAX_COLLECTION_NAME_LENGTH;

use crate::context::Context;
use crate::database::create_collection::{CreateCollectionError, CreateCollectionOptions};
use crate::http::constants::CREATE_COLLECTION_REQUEST_MAX_BYTES;
use crate::http::errors::HttpError;

use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};

use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::ContentTypeValidation;

use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCollectionRequestJsonData {
    collection_name: String,
    is_manual: bool,
    // Only for manual collections
    initial_generation_id: Option<EncodedGenerationIdJsonData>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    generation_id: EncodedGenerationIdJsonData,
}

fn handler(options: StaticRouteOptions) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_utf8_json_by_default()?;

        let body = read_limited_body(request, CREATE_COLLECTION_REQUEST_MAX_BYTES).await?;
        let data: CreateCollectionRequestJsonData = read_json(body)?;

        let collection_name = data.collection_name;
        let is_manual = data.is_manual;

        if collection_name.len() > MAX_COLLECTION_NAME_LENGTH {
            return Err(HttpError::Generic400("collectionName cannot be > 512"));
        }

        let initial_generation_id =
            EncodedGenerationIdJsonData::decode_opt(data.initial_generation_id)?;

        if initial_generation_id.is_some() != is_manual {
            return Err(HttpError::Generic400(
                "initialGenerationId should be present if isManual and be absent if !isManual",
            ));
        }

        let result = context
            .database
            .create_collection(&collection_name, CreateCollectionOptions { is_manual })
            .await;

        let collection = match result {
            Ok(collection) => collection,
            Err(err) => {
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
        };

        let generation_id = collection.get_generation_id().await;

        let response = ResponseJsonData {
            generation_id: EncodedGenerationIdJsonData::encode(
                generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
        };

        create_ok_json_response(&response)
    })
}

pub fn register_create_collection_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/collections/", handler);
}
