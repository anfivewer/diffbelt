use crate::context::Context;
use crate::http::routing::response::{Response, StringResponse};
use crate::http::routing::{Request, StaticRouteFnResult, StaticRouteOptions};

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        Ok(Response::String(StringResponse {
            base: Default::default(),
            str: format!("This is put, method: {}", options.request.method()),
        }))
    })
}

pub fn register_put_route(context: &mut Context) {
    context.routing.add_static_get_route("/put", handler);
}
