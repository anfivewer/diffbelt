use crate::context::Context;
use crate::http::routing::response::{Response, StringResponse};
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};

fn root_handle(_options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        Ok(Response::String(StringResponse {
            base: Default::default(),
            str: format!("Hello, World!"),
        }))
    })
}

pub fn register_root_route(context: &mut Context) {
    context.routing.add_static_get_route("/", root_handle);
}
