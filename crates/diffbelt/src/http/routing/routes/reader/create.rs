use std::sync::Arc;

use crate::collection::methods::create_reader::CreateReaderOptions;
use crate::collection::Collection;
use crate::common::OwnedGenerationId;
use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;
use crate::http::data::bytes_generation_id::flatbuffers_generation_id_to_owned_generation_id_opt;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_decode_opt, EncodedGenerationIdJsonData,
};
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::request::Request;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::HttpHandlerResult;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::{
    create_ok_flatbuffers_response, create_ok_no_error_json_response,
};
use crate::http::validation::ContentTypeValidation;
use diffbelt_protos::protos::api::readers::{CreateReaderResponseArgs, ListReadersResponseArgs};
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, CreateReaderApiHandler, ListReadersApiHandler,
};
use diffbelt_protos::Serializer;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    reader_name: String,
    generation_id: Option<EncodedGenerationIdJsonData>,
    collection_name: Option<String>,
}

struct UnifiedRequestData {
    reader_name: String,
    reader_collection_name: Option<String>,
    generation_id: Option<OwnedGenerationId>,
}

pub async fn unified_handler(
    collection: Arc<Collection>,
    request_context: RequestContext,
    data: UnifiedRequestData,
) -> HttpHandlerResult {
    let UnifiedRequestData {
        reader_name,
        reader_collection_name,
        generation_id,
    } = data;

    let options = CreateReaderOptions {
        reader_name,
        collection_name: reader_collection_name,
        generation_id,
    };

    let result = collection.create_reader(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/create error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    if request_context.is_flatbuffers() {
        let serializer = Serializer::new();
        let response =
            CreateReaderApiHandler::create_response(serializer, CreateReaderResponseArgs {});

        create_ok_flatbuffers_response(response)
    } else {
        create_ok_no_error_json_response()
    }
}

pub async fn create_reader(
    request: impl Request,
    request_context: RequestContext,
    collection: Arc<Collection>,
) -> HttpHandlerResult {
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, READER_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let generation_id = encoded_generation_id_data_decode_opt(data.generation_id)?;

    let data = UnifiedRequestData {
        reader_name: data.reader_name,
        reader_collection_name: data.collection_name,
        generation_id,
    };

    unified_handler(collection, request_context, data).await
}

pub async fn create_reader_flatbuffers_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, CreateReaderApiHandler>,
) -> HttpHandlerResult {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::Generic400("no collection name"))?;

    let collection = context
        .database
        .get_collection(collection_name)
        .await
        .ok_or_else(|| HttpError::NoSuchCollection)?;

    let reader = request
        .reader()
        .ok_or_else(|| HttpError::Generic400("no reader"))?;
    let reader_name = reader
        .reader_name()
        .ok_or_else(|| HttpError::Generic400("no reader name"))?
        .to_string();
    let reader_collection_name = reader.collection_name().map(|x| x.to_string());
    let generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(reader.generation_id())?;

    let data = UnifiedRequestData {
        reader_name,
        reader_collection_name,
        generation_id,
    };

    unified_handler(collection, request_context, data).await
}
