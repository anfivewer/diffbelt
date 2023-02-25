use crate::context::Context;
use std::ops::Deref;
use std::time::Duration;

use crate::http::errors::HttpError;
use crate::http::request::Request;

use crate::http::routing::{PatternRouteFnResult, PatternRouteOptions};
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};

use crate::common::{IsByteArray, OwnedGenerationId};
use crate::http::custom_errors::no_such_collection_error;
use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;
use crate::http::routing::response::Response;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::MethodsValidation;
use crate::util::str_serialization::StrSerializationType;
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

fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> PatternRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;
        let collection_id = options.groups.0;

        request.allow_only_methods(&["GET"])?;

        let result = context.database.get_collection(&collection_id).await;

        let Some(collection) = result else {
            return Err(no_such_collection_error());
        };

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
            // If no `generationId` passed, response with current generationId
            let id = collection.get_generation_id().await;
            return make_response(id)
        };

        let encoding = StrSerializationType::from_opt_str(generation_id_encoding)
            .map_err(|_| HttpError::Generic400("invalid encoding"))?;

        let generation_id = encoding
            .deserialize(generation_id)
            .map_err(|_| HttpError::Generic400("invalid encoded value"))?;

        let generation_id = OwnedGenerationId::from_boxed_slice(generation_id)
            .map_err(|_| HttpError::Generic400("invalid generation_id"))?;

        let mut time_left = Duration::from_millis(60 * 1000);

        let mut generation_id_receiver = collection.get_generation_id_receiver().clone();

        let new_generation_id = loop {
            {
                let new_generation_id = generation_id_receiver.borrow_and_update();
                let new_generation_id = new_generation_id.deref();

                if new_generation_id != &generation_id {
                    break Some(new_generation_id.clone());
                }
            };

            let changed_fut = generation_id_receiver.changed();
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

        let id = new_generation_id.unwrap_or(generation_id);
        make_response(id)
    })
}

pub fn register_collection_generation_id_stream_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/generationId/stream$").unwrap(),
        id_only_group,
        handler,
    );
}

fn make_response(id: OwnedGenerationId) -> Result<Response, HttpError> {
    let generation_id =
        EncodedGenerationIdJsonData::encode(id.as_ref(), StrSerializationType::Utf8);

    create_ok_json_response(&CollectionGenerationIdStreamResponseJsonData { generation_id })
}
