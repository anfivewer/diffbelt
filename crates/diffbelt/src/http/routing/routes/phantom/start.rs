use diffbelt_macro::fn_box_pin_async;
use regex::Regex;
use serde::Serialize;
use serde_with::skip_serializing_none;
use diffbelt_types::common::phantom_id::EncodedPhantomIdJsonData;

use crate::context::Context;
use crate::http::data::encoded_phantom_id::EncodedPhantomIdJsonDataTrait;

use crate::http::errors::HttpError;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};

use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::get_collection::get_collection;

use crate::http::util::response::create_ok_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    phantom_id: EncodedPhantomIdJsonData,
}

#[fn_box_pin_async]
async fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let collection = get_collection(&context, &collection_name).await?;

    let result = collection.start_phantom().await;

    let phantom_id = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("phantom/start error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = ResponseJsonData {
        phantom_id: EncodedPhantomIdJsonData::new(phantom_id, StrSerializationType::Utf8),
    };

    create_ok_json_response(&response)
}

pub fn register_start_phantom_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/phantom/start$").unwrap(),
        id_only_group,
        handler,
    );
}
