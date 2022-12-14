use crate::context::Context;
use crate::routes::{Response, StaticRouteOptions, StringResponse};
use futures::future::BoxFuture;

fn test_handle(_options: StaticRouteOptions) -> BoxFuture<'static, Response> {
    Box::pin(async move {
        return Response::String(StringResponse {
            base: Default::default(),
            str: format!("Counter is broken"),
        });
    })
}

pub fn register_root_route(context: &mut Context) {
    context.routing.add_static_get_route("/", test_handle);
}
