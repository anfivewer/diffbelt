use crate::collection::methods::update_reader::UpdateReaderOptions;
use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Deserialize;

use crate::context::Context;
use crate::http::constants::READER_REQUEST_MAX_BYTES;

use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::routes::reader::delete::delete_reader;
use crate::http::routing::routes::reader::update::update_reader;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{
    id_only_group, id_with_name_group, IdOnlyGroup, IdWithNameGroup,
};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestJsonData {
    reader_name: String,
    generation_id: Option<EncodedGenerationIdJsonData>,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdWithNameGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.id;
    let reader_name = options.groups.name;

    let collection = get_collection(&context, &collection_name).await?;

    match request.method() {
        "PUT" => update_reader(request, collection, reader_name).await,
        "DELETE" => delete_reader(request, collection, reader_name).await,
        _ => Err(HttpError::MethodNotAllowed),
    }
}

pub fn register_reader_by_name_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/readers/(?P<name>[^/]+)$").unwrap(),
        id_with_name_group,
        handler,
    );
}
