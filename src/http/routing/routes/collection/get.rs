use crate::collection::Collection;
use crate::common::IsByteArray;
use crate::http::errors::HttpError;
use crate::http::routing::response::Response;
use crate::http::util::response::create_ok_json_response;
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::sync::Arc;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetCollectionResponseJsonData {
    generation_id: Option<String>,
    generation_id_encoding: Option<String>,
}

pub async fn get_collection(collection: Arc<Collection>) -> Result<Response, HttpError> {
    let generation_id = collection.get_generation_id().await;

    let (generation_id, generation_id_encoding) =
        StrSerializationType::Utf8.serialize_with_priority(generation_id.get_byte_array());

    create_ok_json_response(&GetCollectionResponseJsonData {
        generation_id: Some(generation_id),
        generation_id_encoding: generation_id_encoding.to_optional_string(),
    })
}
