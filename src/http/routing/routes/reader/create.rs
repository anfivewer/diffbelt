use crate::collection::methods::create_reader::CreateReaderOptions;
use std::sync::Arc;

use crate::collection::Collection;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::Response;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::json::serde::deserialize_strict_null;
use crate::util::str_serialization::StrSerializationType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    reader_name: String,
    generation_id: Option<EncodedGenerationIdJsonData>,
    collection_name: Option<String>,
}

pub async fn create_reader(
    request: impl Request,
    collection: Arc<Collection>,
) -> Result<Response, HttpError> {
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, READER_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let RequestJsonData {
        reader_name,
        generation_id,
        collection_name: reader_collection_name,
    } = data;

    let generation_id = EncodedGenerationIdJsonData::decode_opt(generation_id)?;

    let options = CreateReaderOptions {
        reader_name,
        collection_name: reader_collection_name,
        generation_id,
    };

    let result = collection.create_reader(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/create error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}
