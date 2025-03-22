use crate::collection::methods::query::ReadQueryCursorOptions;
use crate::collection::Collection;
use crate::context::Context;
use crate::http::data::bytes_generation_id::{
    flatbuffers_generation_id_to_owned_generation_id_opt,
    flatbuffers_phantom_id_to_owned_phantom_id_opt,
};
use crate::http::data::query_response::QueryResponseJsonData;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::request::Request;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::HttpHandlerResult;
use crate::http::util::get_collection::get_collection;
use crate::http::util::query::serialize_query_ok;
use crate::http::util::response::{create_ok_flatbuffers_response, create_ok_json_response};
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, NextQueryApiHandler, StartQueryApiHandler,
};
use diffbelt_protos::Serializer;
use std::sync::Arc;
use tracing::debug;

struct UnifiedRequestData {
    collection: Arc<Collection>,
    cursor_id: Box<str>,
}

async fn unified_handler(
    request_context: RequestContext,
    data: UnifiedRequestData,
) -> HttpHandlerResult {
    let options = ReadQueryCursorOptions {
        cursor_id: data.cursor_id,
    };
    let result = data.collection.read_query_cursor(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("query/start error {:?}", err);
            debug!(parent: &request_context.span, "error: {err:?}");
            return Err(HttpError::Unspecified);
        }
    };

    if request_context.is_flatbuffers() {
        let mut serializer = Serializer::new();
        let response = serialize_query_ok(&mut serializer, result);
        let response = NextQueryApiHandler::create_response(serializer, response);
        create_ok_flatbuffers_response(response)
    } else {
        let response = QueryResponseJsonData::from(result);
        create_ok_json_response(&response)
    }
}

pub async fn json_read_cursor(
    request_context: RequestContext,
    collection: Arc<Collection>,
    cursor_id: Box<str>,
) -> HttpHandlerResult {
    let data = UnifiedRequestData {
        collection,
        cursor_id,
    };

    unified_handler(request_context, data).await
}

pub async fn next_query_flatbuffers_route(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, NextQueryApiHandler>,
) -> HttpHandlerResult {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::Generic400("no collection name"))?;
    let collection = get_collection(&context, collection_name).await?;
    let cursor_id = request
        .cursor_id()
        .ok_or_else(|| HttpError::Generic400("no cursor id"))?;
    let cursor_id = Box::from(cursor_id);

    let data = UnifiedRequestData {
        collection,
        cursor_id,
    };

    unified_handler(request_context, data).await
}
