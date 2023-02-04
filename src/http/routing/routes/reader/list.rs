use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

// TODO: add filter by name to use as `get/getMany`
fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let _context = options.context;
        let request = options.request;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        Err(HttpError::Unspecified)
    })
}

pub fn register_list_readers_route(context: &mut Context) {
    context
        .routing
        .add_static_post_route("/reader/list", handler);
}
