use std::sync::Arc;

use crate::collection::methods::list_readers::ListReadersOk;
use crate::collection::Collection;
use crate::common::IsByteArray;
use crate::context::Context;
use crate::http::data::reader_record::ReaderRecordJsonData;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::request::Request;
use crate::http::routing::response::HttpResponse;
use crate::http::routing::HttpHandlerResult;
use crate::http::util::response::{create_ok_flatbuffers_response, create_ok_json_response};
use diffbelt_protos::protos::api::readers::{
    ListReadersResponseArgs, ReaderRecord, ReaderRecordArgs,
};
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, ListReadersApiHandler,
};
use diffbelt_protos::Serializer;
use serde::Serialize;
use serde_with::skip_serializing_none;
use tracing::error;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    items: Vec<ReaderRecordJsonData>,
}

async fn unified_handler(
    collection: Arc<Collection>,
    request_context: RequestContext,
) -> HttpHandlerResult {
    let result = collection.list_readers().await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            error!(parent: &request_context.span, "list_readers() error: {err:?}");
            return Err(HttpError::Unspecified);
        }
    };

    let ListReadersOk { items } = result;

    if request_context.is_flatbuffers() {
        let mut serializer = Serializer::new();
        let readers: Vec<_> = items
            .into_iter()
            .map(|reader| {
                let reader_name = Some(serializer.create_string(&reader.reader_name));
                let collection_name = reader
                    .collection_name
                    .map(|name| serializer.create_string(&name));
                let generation_id = reader
                    .generation_id
                    .map(|x| serializer.create_vector(x.get_byte_array()));
                ReaderRecord::create(
                    serializer.buffer_builder(),
                    &ReaderRecordArgs {
                        reader_name,
                        collection_name,
                        generation_id,
                    },
                )
            })
            .collect();
        let readers = Some(serializer.create_vector(&readers));
        let response =
            ListReadersApiHandler::create_response(serializer, ListReadersResponseArgs { readers });

        create_ok_flatbuffers_response(response)
    } else {
        let response = ResponseJsonData {
            items: ReaderRecordJsonData::encode_vec(items),
        };

        create_ok_json_response(&response)
    }
}

pub async fn list_readers(
    request_context: RequestContext,
    collection: Arc<Collection>,
) -> HttpHandlerResult {
    unified_handler(collection, request_context).await
}

pub async fn list_readers_flatbuffers_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, ListReadersApiHandler>,
) -> HttpHandlerResult {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::Generic400("no collection name"))?;

    let collection = context
        .database
        .get_collection(collection_name)
        .await
        .ok_or_else(|| HttpError::NoSuchCollection)?;

    unified_handler(collection, request_context).await
}
