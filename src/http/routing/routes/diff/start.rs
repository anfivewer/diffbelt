use crate::collection::methods::diff::DiffOptions;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::common::generation_id::GenerationIdSource;
use crate::common::reader::ReaderDef;
use crate::common::OwnedGenerationId;
use crate::context::Context;
use crate::http::constants::QUERY_START_REQUEST_MAX_BYTES;
use crate::http::data::diff_response::DiffResponseJsonData;
use crate::http::data::encoded_generation_id::EncodedOptionalGenerationIdFlatJsonData;

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    #[serde(flatten)]
    from_generation_id: EncodedOptionalGenerationIdFlatJsonData,
    #[serde(flatten)]
    to_generation_id: EncodedOptionalGenerationIdFlatJsonData,

    reader_id: Option<String>,
    reader_collection_name: Option<String>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_id = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, QUERY_START_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let from_generation_id = data.from_generation_id.decode(&decoder)?;
    let to_generation_id = data.to_generation_id.decode(&decoder)?;
    let reader_id = data.reader_id;
    let reader_collection_name = data.reader_collection_name;

    let from_generation_id =
        into_from_generation_id_source(from_generation_id, reader_id, reader_collection_name)?;
    let collection = get_collection(&context, &collection_id).await?;

    let options = DiffOptions {
        from_generation_id,
        to_generation_id_loose: to_generation_id,
    };

    let result = collection.diff(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("query/diff error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = DiffResponseJsonData::from(result);
    create_ok_json_response(&response)
}

fn into_from_generation_id_source(
    from_generation_id: Option<OwnedGenerationId>,
    reader_id: Option<String>,
    reader_collection_name: Option<String>,
) -> Result<GenerationIdSource, HttpError> {
    match from_generation_id {
        Some(generation_id) => return Ok(GenerationIdSource::Value(Some(generation_id))),
        None => {}
    }

    let Some(reader_id) = reader_id else {
        return Err(HttpError::Generic400("either fromGenerationId or readerId should be present"));
    };

    Ok(GenerationIdSource::Reader(ReaderDef {
        collection_id: reader_collection_name,
        reader_id,
    }))
}

pub fn register_start_diff_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/diff/start$").unwrap(),
        id_only_group,
        handler,
    );
}
