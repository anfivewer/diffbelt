use crate::context::Context;
use crate::routes::{BaseResponse, Response, StaticRouteOptions, StringResponse};
use futures::future::BoxFuture;

fn test_handle(options: StaticRouteOptions) -> BoxFuture<'static, Response> {
    return Box::pin(async move {
        let context = options.context.read().await;
        let database = context.meta_raw_db.clone();
        drop(context);

        let value = database.next_value().await;

        let value = match value {
            Ok(value) => value,
            Err(_) => {
                return Response::String(StringResponse {
                    base: BaseResponse { status: 500 },
                    str: "Database error".to_string(),
                });
            }
        };

        return Response::String(StringResponse {
            base: Default::default(),
            str: format!("Counter is {}", value),
        });
    });
}

pub fn register_root_route(context: &mut Context) {
    context.routing.add_static_get_route("/", test_handle);
}
