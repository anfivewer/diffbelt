use crate::context::Context;
use crate::http::constants::FLATBUFFERS_REQUEST_MAX_BYTES;
use crate::http::errors::HttpError;
use crate::http::routing::routes::collection::create::create_collection_flatbuffers_route;
use crate::http::routing::routes::collection::generation_id_stream::generation_id_stream_flatbuffers_route;
use crate::http::routing::routes::generation::start::start_generation_flatbuffers_route;
use crate::http::routing::routes::get::get_flatbuffers_route;
use crate::http::routing::routes::get_keys_around::get_keys_around_flatbuffers_route;
use crate::http::routing::routes::put_many::put_many_flatbuffers_route;
use crate::http::routing::routes::query::next::next_query_flatbuffers_route;
use crate::http::routing::routes::query::start::start_query_flatbuffers_route;
use crate::http::routing::routes::reader::create::create_reader_flatbuffers_handler;
use crate::http::routing::routes::reader::list::list_readers_flatbuffers_handler;
use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};
use crate::http::util::read_body::read_limited_aligned_bytes;
use crate::http::validation::MethodsValidation;
use diffbelt_protos::deserialize;
use diffbelt_protos::protos::api::methods::RequestBody;
use diffbelt_protos::protos::handlers::{
    ApiHandler, CreateCollectionApiHandler, CreateReaderApiHandler, GenerationIdStreamApiHandler,
    GetApiHandler, GetKeysAroundApiHandler, ListReadersApiHandler, NextQueryApiHandler,
    PutManyApiHandler, StartGenerationApiHandler, StartQueryApiHandler,
};
use diffbelt_protos::protos::impls::RequestProto;
use diffbelt_util_no_std::option::store_in_option;
use tracing::trace;

fn handler(options: StaticRouteOptions) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        let request = options.request;

        request.allow_only_methods(&["POST"])?;

        let bytes = read_limited_aligned_bytes(request, FLATBUFFERS_REQUEST_MAX_BYTES).await?;
        let serialized = deserialize::<RequestProto>(bytes.as_ref())
            .map_err(|err| HttpError::InvalidFlatbuffers(err.to_string()))?;

        options.request_context.set_flatbuffers();
        options.request_context.span.in_scope(|| {
            trace!(
                "method:{}",
                serialized.body_type().variant_name().unwrap_or("?")
            );
        });

        macro_rules! body_handler {
            ($api_handler:ident, $handler:ident) => {{
                let data = $api_handler::request(&serialized)
                    .ok_or_else(|| HttpError::GenericFlatbuffers400("no request data"))?;
                return $handler(options.context, options.request_context, data).await;
            }};
        }

        match serialized.body_type() {
            RequestBody::CreateCollection => body_handler!(
                CreateCollectionApiHandler,
                create_collection_flatbuffers_route
            ),
            RequestBody::StartGeneration => body_handler!(
                StartGenerationApiHandler,
                start_generation_flatbuffers_route
            ),
            RequestBody::PutMany => body_handler!(PutManyApiHandler, put_many_flatbuffers_route),
            RequestBody::GenerationIdStream => body_handler!(
                GenerationIdStreamApiHandler,
                generation_id_stream_flatbuffers_route
            ),
            RequestBody::ListReaders => {
                body_handler!(ListReadersApiHandler, list_readers_flatbuffers_handler)
            }
            RequestBody::CreateReader => {
                body_handler!(CreateReaderApiHandler, create_reader_flatbuffers_handler)
            }
            RequestBody::StartQuery => {
                body_handler!(StartQueryApiHandler, start_query_flatbuffers_route)
            }
            RequestBody::NextQuery => {
                body_handler!(NextQueryApiHandler, next_query_flatbuffers_route)
            }
            RequestBody::Get => {
                body_handler!(GetApiHandler, get_flatbuffers_route)
            }
            RequestBody::GetKeysAround => {
                body_handler!(GetKeysAroundApiHandler, get_keys_around_flatbuffers_route)
            }
            body => {
                let mut variant_name = None;
                let variant_name = match body.variant_name() {
                    Some(name) => name,
                    None => {
                        let variant_name =
                            store_in_option(&mut variant_name, format!("#{}", body.0));
                        variant_name.as_str()
                    }
                };
                return Err(HttpError::InvalidFlatbuffers(format!(
                    "Unsupported flatbuffers request: {}",
                    variant_name
                )));
            }
        }
    })
}

pub fn register_flatbuffers_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/flatbuffers", handler);
}
