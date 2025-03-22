use crate::collection::methods::query::{QueryOk, QueryOptions};
use crate::common::{IsByteArray, OwnedGenerationId, OwnedPhantomId};
use crate::context::Context;
use crate::http::constants::QUERY_START_REQUEST_MAX_BYTES;
use crate::http::data::bytes_generation_id::{
    flatbuffers_generation_id_to_owned_generation_id_opt,
    flatbuffers_phantom_id_to_owned_phantom_id_opt,
};
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_decode_opt, EncodedGenerationIdJsonData,
};
use crate::http::data::encoded_phantom_id::EncodedPhantomIdJsonDataTrait;
use crate::http::data::query_response::QueryResponseJsonData;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::query::serialize_query_ok;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::{create_ok_flatbuffers_response, create_ok_json_response};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use diffbelt_protos::protos::api::put_many::PutManyResponseArgs;
use diffbelt_protos::protos::api::query::QueryResponseArgs;
use diffbelt_protos::protos::handlers::{ApiHandler, ApiHandlerRequestFlatType, NextQueryApiHandler, PutManyApiHandler, StartQueryApiHandler};
use diffbelt_protos::protos::impls::StartQueryRequestProto;
use diffbelt_protos::Serializer;
use diffbelt_types::common::phantom_id::EncodedPhantomIdJsonData;
use regex::Regex;
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    generation_id: Option<EncodedGenerationIdJsonData>,
    phantom_id: Option<EncodedPhantomIdJsonData>,
}

struct UnifiedRequestData<'a> {
    collection_name: &'a str,
    generation_id: Option<OwnedGenerationId>,
    phantom_id: Option<OwnedPhantomId>,
}

async fn unified_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'_>,
) -> HttpHandlerResult {
    let collection = get_collection(&context, data.collection_name).await?;

    let options = QueryOptions {
        generation_id: data.generation_id,
        phantom_id: data.phantom_id,
    };

    let result = collection.query(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            debug!(parent: &request_context.span, "error: {err:?}");
            return Err(HttpError::Unspecified);
        }
    };

    if request_context.is_flatbuffers() {
        let mut serializer = Serializer::new();
        let response = serialize_query_ok(&mut serializer, result);
        let response = StartQueryApiHandler::create_response(serializer, response);
        create_ok_flatbuffers_response(response)
    } else {
        let response = QueryResponseJsonData::from(result);
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

    let body = read_limited_body(request, QUERY_START_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let generation_id = encoded_generation_id_data_decode_opt(data.generation_id)?;
    let phantom_id = EncodedPhantomIdJsonData::decode_opt(data.phantom_id, &decoder)?;

    let data = UnifiedRequestData {
        collection_name: &collection_name,
        generation_id,
        phantom_id,
    };

    unified_handler(context, options.request_context, data).await
}

pub fn register_start_query_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/query/$").unwrap(),
        id_only_group,
        json_handler,
    );
}

pub async fn start_query_flatbuffers_route(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, StartQueryApiHandler>,
) -> HttpHandlerResult {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::Generic400("no collection name"))?;
    let generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(request.generation_id())?;
    let phantom_id = flatbuffers_phantom_id_to_owned_phantom_id_opt(request.phantom_id())?;

    let data = UnifiedRequestData {
        collection_name,
        generation_id,
        phantom_id,
    };

    unified_handler(context, request_context, data).await
}
