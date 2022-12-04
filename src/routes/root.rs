use crate::context::Context;
use futures::future::BoxFuture;
use crate::routes::{StaticRouteOptions, Response, StringResponse, BaseResponse};

fn test_handle(options: StaticRouteOptions<'_>) -> BoxFuture<Response> {
    return Box::pin(async move {
        let context = options.context.lock().await;
        let database = context.raw_db.clone();
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
