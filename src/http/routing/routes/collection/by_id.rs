use crate::context::Context;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::routes::collection::delete::delete_collection;
use crate::http::routing::routes::collection::get::get_collection;
use crate::http::routing::{PatternRouteFnResult, PatternRouteOptions};
use crate::http::util::id_group::{id_only_group, IdOnlyGroup};

use regex::Regex;

fn handler(options: PatternRouteOptions<IdOnlyGroup>) -> PatternRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;
        let collection_id = options.groups.0;

        let result = context.database.get_collection(&collection_id).await;

        let Some(collection) = result else {
            return Err(HttpError::Generic400("no such collection"));
        };

        match request.method() {
            "GET" => get_collection(request, collection).await,
            "DELETE" => delete_collection(collection).await,
            _ => Err(HttpError::MethodNotAllowed),
        }
    })
}

pub fn register_collection_by_id_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)$").unwrap(),
        id_only_group,
        handler,
    );
}