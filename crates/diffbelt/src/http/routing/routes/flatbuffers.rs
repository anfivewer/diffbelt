use crate::context::Context;
use crate::http::constants::FLATBUFFERS_REQUEST_MAX_BYTES;
use crate::http::errors::HttpError;
use crate::http::routing::routes::collection::create::create_collection_flatbuffers_route;
use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};
use crate::http::util::read_body::read_limited_aligned_bytes;
use crate::http::validation::MethodsValidation;
use diffbelt_protos::deserialize;
use diffbelt_protos::protos::handlers::{ApiHandler, CreateCollectionApiHandler};
use diffbelt_protos::protos::impls::RequestProto;
use tracing::trace;

fn handler(options: StaticRouteOptions) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["POST"])?;

        let bytes = read_limited_aligned_bytes(request, FLATBUFFERS_REQUEST_MAX_BYTES).await?;
        let serialized = deserialize::<RequestProto>(bytes.as_ref())
            .map_err(|err| HttpError::InvalidFlatbuffers(err.to_string()))?;

        options.span.in_scope(|| {
            trace!(
                "method:{}",
                serialized.body_type().variant_name().unwrap_or("?")
            );
        });

        let data = CreateCollectionApiHandler::request(&serialized)
            .ok_or_else(|| HttpError::GenericFlatbuffers400("no request data"))?;

        create_collection_flatbuffers_route(context, data).await
    })
}

pub fn register_flatbuffers_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/flatbuffers", handler);
}
