use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use crate::common::{GenerationId, IsByteArray, OwnedGenerationId};
use crate::context::Context;
use crate::http::data::bytes_generation_id::flatbuffers_generation_id_to_owned_generation_id_opt;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_encode, EncodedGenerationIdJsonData,
};
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::request::Request;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::response::{create_ok_flatbuffers_response, create_ok_json_response};
use crate::http::validation::MethodsValidation;
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use diffbelt_protos::protos::api::generation::GenerationIdStreamResponseArgs;
use diffbelt_protos::protos::api::put_many::PutManyResponseArgs;
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, GenerationIdStreamApiHandler, PutManyApiHandler,
};
use diffbelt_protos::Serializer;
use regex::Regex;
use serde::Serialize;
use serde_with::skip_serializing_none;
use tokio::select;
use tokio::time::{sleep, Instant};

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionGenerationIdStreamResponseJsonData {
    generation_id: EncodedGenerationIdJsonData,
}

struct UnifiedRequestData<'a> {
    collection_name: &'a str,
    generation_id: Option<GenerationId<'a>>,
    to_generation_id: Option<GenerationId<'a>>,
}

async fn unified_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'_>,
) -> HttpHandlerResult {
    let UnifiedRequestData {
        collection_name,
        generation_id,
        to_generation_id,
    } = data;

    let result = context.database.get_collection(&collection_name).await;

    let Some(collection) = result else {
        return Err(HttpError::NoSuchCollection);
    };

    //

    if generation_id.is_none() && to_generation_id.is_none() {
        // If no `generation_id` or `to_generation_id` passed, response with current generationId
        let id = collection.generation_pair().generation_id;
        return make_response(id);
    };

    let mut time_left = Duration::from_millis(60 * 1000);

    let mut generation_pair_receiver = collection.generation_pair_receiver.clone();

    let new_generation_id = loop {
        {
            let new_generation_pair = generation_pair_receiver.borrow_and_update();
            let new_generation_id = &new_generation_pair.deref().generation_id;

            if let Some(to_generation_id) = to_generation_id {
                if new_generation_id.as_ref() >= to_generation_id {
                    break Some(new_generation_id.clone());
                }
            }

            if let Some(generation_id) = generation_id {
                if new_generation_id.as_ref() != generation_id {
                    break Some(new_generation_id.clone());
                }
            }
        };

        let changed_fut = generation_pair_receiver.changed();
        let timeout_fut = sleep(time_left);

        // Measure elapsed time
        let now = Instant::now();

        select! {
            // Wait for value update
            result = changed_fut => {
                result.map_err(|_| HttpError::PublicInternal500("gen_id_receiver"))?
            },
            _ = timeout_fut => {
                // timeout
                break None;
            },
        }

        time_left -= now.elapsed();

        if time_left <= Duration::ZERO {
            break None;
        }
    };

    let id = new_generation_id
        .or_else(|| generation_id.map(|x| x.to_owned()))
        .unwrap_or_else(|| collection.generation_pair().generation_id);

    if request_context.is_flatbuffers() {
        let mut serializer = Serializer::new();
        let generation_id = Some(serializer.create_vector(id.get_byte_array()));
        let response = GenerationIdStreamApiHandler::create_response(
            serializer,
            GenerationIdStreamResponseArgs { generation_id },
        );

        create_ok_flatbuffers_response(response)
    } else {
        make_response(id)
    }
}

fn make_response(id: OwnedGenerationId) -> Result<HttpResponse, HttpError> {
    let generation_id = encoded_generation_id_data_encode(id.as_ref(), StrSerializationType::Utf8);

    create_ok_json_response(&CollectionGenerationIdStreamResponseJsonData { generation_id })
}

#[fn_box_pin_async]
async fn json_handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["GET"])?;

    let params = request
        .query_params()
        .map_err(|_| HttpError::Generic400("invalidQueryParams"))?;

    let mut generation_id = None;
    let mut generation_id_encoding = None;

    for (key, value) in params {
        match key.deref() {
            "generationId" => {
                generation_id = Some(value);
            }
            "generationIdEncoding" => {
                generation_id_encoding = Some(value);
            }
            _ => {}
        }
    }

    let Some(generation_id) = generation_id else {
        return unified_handler(
            context,
            options.request_context,
            UnifiedRequestData {
                collection_name: &collection_name,
                generation_id: None,
                to_generation_id: None,
            },
        )
        .await;
    };

    let encoding = StrSerializationType::from_opt_str(generation_id_encoding)
        .map_err(|_| HttpError::Generic400("invalid encoding"))?;

    let generation_id = encoding
        .deserialize(generation_id)
        .map_err(|_| HttpError::Generic400("invalid encoded value"))?;

    let generation_id = OwnedGenerationId::from_boxed_slice(generation_id)
        .map_err(|_| HttpError::Generic400("invalid generation_id"))?;

    unified_handler(
        context,
        options.request_context,
        UnifiedRequestData {
            collection_name: &collection_name,
            generation_id: Some(generation_id.as_ref()),
            to_generation_id: None,
        },
    )
    .await
}

pub fn register_collection_generation_id_stream_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/generationId/stream$").unwrap(),
        id_only_group,
        json_handler,
    );
}

pub async fn generation_id_stream_flatbuffers_route<'a>(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'a, GenerationIdStreamApiHandler>,
) -> Result<HttpResponse, HttpError> {
    let generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(request.generation_id())?;
    let to_generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(request.to_generation_id())?;

    let data = UnifiedRequestData {
        collection_name: request
            .collection_name()
            .ok_or_else(|| HttpError::GenericFlatbuffers400("no collection_name"))?,
        generation_id: generation_id.as_ref().map(|x| x.as_ref()),
        to_generation_id: to_generation_id.as_ref().map(|x| x.as_ref()),
    };

    unified_handler(context, request_context, data).await
}
