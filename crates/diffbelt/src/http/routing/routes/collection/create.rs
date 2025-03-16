use crate::common::constants::MAX_COLLECTION_NAME_LENGTH;
use crate::common::IsByteArray;
use crate::context::Context;
use crate::database::create_collection::{CreateCollectionError, CreateCollectionOptions};
use crate::http::constants::CREATE_COLLECTION_REQUEST_MAX_BYTES;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_encode, EncodedGenerationIdJsonData,
};
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::{create_ok_flatbuffers_response, create_ok_json_response};
use crate::http::validation::ContentTypeValidation;
use crate::util::str_serialization::StrSerializationType;
use diffbelt_protos::protos::api::collection::CreateCollectionResponseArgs;
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, CreateCollectionApiHandler,
};
use diffbelt_protos::{FlatbuffersGenericType, Serializer};
use diffbelt_util_no_std::option::store_in_option;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCollectionRequestJsonData {
    collection_name: String,
    is_manual: bool,
}

struct UnifiedRequestData<'a> {
    is_flatbuffers: bool,
    collection_name: &'a str,
    is_manual: bool,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    generation_id: EncodedGenerationIdJsonData,
}

async fn unified_handler<'a>(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'a>,
) -> Result<HttpResponse, HttpError> {
    let collection_name = data.collection_name;
    let is_manual = data.is_manual;

    if collection_name.len() > MAX_COLLECTION_NAME_LENGTH {
        return Err(HttpError::Generic400("collectionName cannot be > 512"));
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

    let generation_id = collection.generation_pair().generation_id;

    if data.is_flatbuffers {
        let mut serializer = Serializer::new();
        let generation_id = Some(serializer.create_vector(generation_id.get_byte_array()));
        let response = CreateCollectionApiHandler::create_response(
            serializer,
            CreateCollectionResponseArgs { generation_id },
        );

        create_ok_flatbuffers_response(response)
    } else {
        let response = ResponseJsonData {
            generation_id: encoded_generation_id_data_encode(
                generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
        };

        create_ok_json_response(&response)
    }
}

fn json_handler(options: StaticRouteOptions) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_utf8_json_by_default()?;

        let mut json_data = None;

        let data = {
            let body = read_limited_body(request, CREATE_COLLECTION_REQUEST_MAX_BYTES).await?;
            let data: CreateCollectionRequestJsonData = read_json(body)?;
            let data = store_in_option(&mut json_data, data);
            UnifiedRequestData {
                is_flatbuffers: false,
                collection_name: &data.collection_name,
                is_manual: data.is_manual,
            }
        };

        unified_handler(context, options.request_context, data).await
    })
}

pub fn register_create_collection_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/collections/", json_handler);
}

pub async fn create_collection_flatbuffers_route(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, CreateCollectionApiHandler>,
) -> Result<HttpResponse, HttpError> {
    let data = UnifiedRequestData {
        is_flatbuffers: true,
        collection_name: request
            .collection_name()
            .ok_or_else(|| HttpError::GenericFlatbuffers400("no collection_name"))?,
        is_manual: request.is_manual(),
    };

    unified_handler(context, request_context, data).await
}
