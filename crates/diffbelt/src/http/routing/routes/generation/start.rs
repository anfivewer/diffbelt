use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::OwnedGenerationId;
use crate::context::Context;
use crate::http::constants::START_GENERATION_REQUEST_MAX_BYTES;
use crate::http::data::bytes_generation_id::flatbuffers_generation_id_to_owned_generation_id;
use crate::http::data::encoded_generation_id::encoded_generation_id_data_into_generation_id;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions, StaticRouteFnFutureResult};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::get_collection::get_collection;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::{
    create_ok_flatbuffers_response, create_ok_no_error_json_response,
};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use diffbelt_protos::protos::api::generation::StartGenerationResponseArgs;
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, StartGenerationApiHandler,
};
use diffbelt_protos::{FlatbuffersGenericType, Serializer};
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use regex::Regex;
use std::sync::Arc;

struct UnifiedRequestData<'a> {
    collection_name: &'a str,
    generation_id: OwnedGenerationId,
    abort_outdated: bool,
}

async fn unified_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'_>,
) -> HttpHandlerResult {
    let UnifiedRequestData {
        collection_name,
        generation_id,
        abort_outdated,
    } = data;

    let collection = get_collection(&context, &collection_name).await?;

    let options = StartGenerationOptions {
        generation_id,
        abort_outdated,
    };

    let result = collection.start_generation(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("generation/start error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    if request_context.is_flatbuffers() {
        let serializer = Serializer::new();
        let response =
            StartGenerationApiHandler::create_response(serializer, StartGenerationResponseArgs {});

        create_ok_flatbuffers_response(response)
    } else {
        create_ok_no_error_json_response()
    }
}

fn json_handler(options: PatternRouteOptions<IdOnlyGroup>) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;
        let collection_name = options.groups.0;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        let body = read_limited_body(request, START_GENERATION_REQUEST_MAX_BYTES).await?;
        let data: StartGenerationRequestJsonData = read_json(body)?;
        let generation_id = encoded_generation_id_data_into_generation_id(data.generation_id)?;
        let data = UnifiedRequestData {
            collection_name: &collection_name,
            generation_id,
            abort_outdated: data.abort_outdated.unwrap_or(false),
        };

        unified_handler(context, options.request_context, data).await
    })
}

pub fn register_start_generation_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/generation/start$").unwrap(),
        id_only_group,
        json_handler,
    );
}

pub async fn start_generation_flatbuffers_route<'a>(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'a, StartGenerationApiHandler>,
) -> Result<HttpResponse, HttpError> {
    let generation_id = flatbuffers_generation_id_to_owned_generation_id(request.generation_id())?;

    let data = UnifiedRequestData {
        collection_name: request
            .collection_name()
            .ok_or_else(|| HttpError::GenericFlatbuffers400("no collection_name"))?,
        generation_id,
        abort_outdated: false,
    };

    unified_handler(context, request_context, data).await
}
