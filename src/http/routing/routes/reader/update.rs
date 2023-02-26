use crate::collection::methods::update_reader::UpdateReaderOptions;
use std::sync::Arc;

use crate::collection::Collection;
use serde::Deserialize;

use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;

use crate::http::errors::HttpError;
use crate::http::request::Request;

use crate::http::routing::response::Response;

use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::ContentTypeValidation;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    generation_id: Option<EncodedGenerationIdJsonData>,
}

pub async fn update_reader(
    request: impl Request,
    collection: Arc<Collection>,
    reader_name: String,
) -> Result<Response, HttpError> {
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, READER_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let RequestJsonData { generation_id } = data;

    let generation_id = EncodedGenerationIdJsonData::decode_opt(generation_id)?;

    let options = UpdateReaderOptions {
        reader_name,
        generation_id,
    };

    let result = collection.update_reader(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/update error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}
