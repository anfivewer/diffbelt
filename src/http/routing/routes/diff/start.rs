use crate::collection::methods::diff::DiffOptions;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::common::generation_id::GenerationIdSource;
use crate::common::reader::ReaderDef;
use crate::common::OwnedGenerationId;
use crate::context::Context;
use crate::http::constants::DIFF_START_REQUEST_MAX_BYTES;
use crate::http::data::diff_response::DiffResponseJsonData;
use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;
use crate::http::data::reader_record::ReaderDiffFromDefJsonData;

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::get_collection::get_collection;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    from_generation_id: Option<EncodedGenerationIdJsonData>,
    to_generation_id: Option<EncodedGenerationIdJsonData>,

    from_reader: Option<ReaderDiffFromDefJsonData>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, DIFF_START_REQUEST_MAX_BYTES).await?;
    let data: RequestJsonData = read_json(body)?;

    let from_generation_id = EncodedGenerationIdJsonData::decode_opt(data.from_generation_id)?;
    let to_generation_id = EncodedGenerationIdJsonData::decode_opt(data.to_generation_id)?;

    let from_generation_id = into_from_generation_id_source(from_generation_id, data.from_reader)?;
    let collection = get_collection(&context, &collection_name).await?;

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
    reader: Option<ReaderDiffFromDefJsonData>,
) -> Result<GenerationIdSource, HttpError> {
    match from_generation_id {
        Some(generation_id) => return Ok(GenerationIdSource::Value(Some(generation_id))),
        None => {}
    }

    let Some(reader) = reader else {
        return Err(HttpError::Generic400("either fromGenerationId or readerId should be present"));
    };

    Ok(GenerationIdSource::Reader(ReaderDef {
        collection_name: reader.collection_name,
        reader_name: reader.reader_name,
    }))
}

pub fn register_start_diff_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/diff/$").unwrap(),
        id_only_group,
        handler,
    );
}
