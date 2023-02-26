use diffbelt_macro::fn_box_pin_async;
use regex::Regex;

use crate::context::Context;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::routes::reader::delete::delete_reader;
use crate::http::routing::routes::reader::update::update_reader;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_with_name_group, IdWithNameGroup};

use crate::http::util::get_collection::get_collection;

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
