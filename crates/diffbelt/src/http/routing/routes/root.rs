use crate::context::Context;
use crate::http::routing::response::{HttpResponse, StringResponse};
use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};

fn root_handle(_options: StaticRouteOptions) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        Ok(HttpResponse::String(StringResponse {
            base: Default::default(),
            str: format!("Hello, World!"),
        }))
    })
}

pub fn register_root_route(context: &mut Context) {
    context.routing.add_static_post_route("/", root_handle);
}
