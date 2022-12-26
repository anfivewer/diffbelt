use crate::context::Context;
use crate::http::routing::routes::put::register_put_route;
use crate::http::routing::routes::root::register_root_route;

pub fn register_routes(context: &mut Context) {
    register_root_route(context);
    register_put_route(context);
}
