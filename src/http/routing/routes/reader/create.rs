use crate::collection::methods::create_reader::CreateReaderOptions;
use crate::collection::methods::diff::DiffOptions;
use crate::collection::methods::list_readers::ListReadersOk;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::common::generation_id::GenerationIdSource;
use crate::common::reader::ReaderDef;
use crate::common::OwnedGenerationId;
use crate::context::Context;
use crate::http::constants::{QUERY_START_REQUEST_MAX_BYTES, READER_REQUEST_MAX_BYTES};
use crate::http::data::diff_response::DiffResponseJsonData;
use crate::http::data::encoded_generation_id::{
    EncodedNullableGenerationIdFlatJsonData, EncodedOptionalGenerationIdFlatJsonData,
};
use crate::http::data::reader_record::ReaderRecordJsonData;

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::{create_ok_json_response, create_ok_no_error_json_response};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    reader_id: String,
    #[serde(flatten)]
    generation_id: EncodedNullableGenerationIdFlatJsonData,
    collection_name: Option<String>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_id = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, READER_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let RequestJsonData {
        reader_id,
        generation_id,
        collection_name,
    } = data;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);
    let generation_id = generation_id.decode(&decoder)?;

    let collection = get_collection(&context, &collection_id).await?;

    let options = CreateReaderOptions {
        reader_id,
        collection_id: collection_name,
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

pub fn register_create_reader_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/reader/create$").unwrap(),
        id_only_group,
        handler,
    );
}
