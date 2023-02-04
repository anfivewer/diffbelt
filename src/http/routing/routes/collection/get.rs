use crate::collection::Collection;
use crate::common::IsByteArray;
use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::Response;
use crate::http::util::response::create_ok_json_response;
use crate::util::str_serialization::StrSerializationType;

use futures::StreamExt;
use serde::Serialize;
use serde_with::skip_serializing_none;

use std::ops::Deref;

use std::sync::Arc;

struct GenerationIdPart {
    generation_id: Option<String>,
    generation_id_encoding: Option<String>,
}

struct NextGenerationIdPart {
    next_generation_id: Option<Option<String>>,
    next_generation_id_encoding: Option<String>,
}

trait ApplyPart {
    fn apply_part(self, response: &mut GetCollectionResponseJsonData);
}

impl ApplyPart for Option<GenerationIdPart> {
    fn apply_part(self, response: &mut GetCollectionResponseJsonData) {
        let Some(part) = self else {
            return;
        };

        response.generation_id = part.generation_id;
        response.generation_id_encoding = part.generation_id_encoding;
    }
}

impl ApplyPart for Option<NextGenerationIdPart> {
    fn apply_part(self, response: &mut GetCollectionResponseJsonData) {
        let Some(part) = self else {
            return;
        };

        response.next_generation_id = part.next_generation_id;
        response.next_generation_id_encoding = part.next_generation_id_encoding;
    }
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetCollectionResponseJsonData {
    generation_id: Option<String>,
    generation_id_encoding: Option<String>,
    next_generation_id: Option<Option<String>>,
    next_generation_id_encoding: Option<String>,
}

pub async fn get_collection(
    request: impl Request,
    collection: Arc<Collection>,
) -> Result<Response, HttpError> {
    let params = request
        .query_params()
        .map_err(|_| HttpError::Generic400("invalidQueryParams"))?;

    let mut with_generation_id = true;
    let mut with_next_generation_id = true;

    for (key, value) in params {
        match key.deref() {
            "fields" => {
                with_generation_id = false;
                with_next_generation_id = false;

                for field in value.split(',') {
                    match field {
                        "generationId" => {
                            with_generation_id = true;
                        }
                        "nextGenerationId" => {
                            with_next_generation_id = true;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let mut response = GetCollectionResponseJsonData {
        generation_id: None,
        generation_id_encoding: None,
        next_generation_id: None,
        next_generation_id_encoding: None,
    };

    type ResponseRef<'a> = &'a mut GetCollectionResponseJsonData;
    type ModifyResponseFn = Box<dyn FnOnce(ResponseRef<'_>) -> () + Send + Sync + 'static>;

    let (generation_id_part, next_generation_id_part) = tokio::join!(
        async {
            if !with_generation_id {
                return None;
            }

            let id = collection.get_generation_id().await;

            let (id, encoding) =
                StrSerializationType::Utf8.serialize_with_priority(id.get_byte_array());

            Some(GenerationIdPart {
                generation_id: Some(id),
                generation_id_encoding: encoding.to_optional_string(),
            })
        },
        async {
            if !with_next_generation_id {
                return None;
            }

            let id = collection.get_next_generation_id().await;
            let Some(id) = id else {
                return None;
            };

            let (id, encoding) =
                StrSerializationType::Utf8.serialize_with_priority(id.get_byte_array());

            Some(NextGenerationIdPart {
                next_generation_id: Some(Some(id)),
                next_generation_id_encoding: encoding.to_optional_string(),
            })
        }
    );

    generation_id_part.apply_part(&mut response);
    next_generation_id_part.apply_part(&mut response);

    create_ok_json_response(&response)
}
