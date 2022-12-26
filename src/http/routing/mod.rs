use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::response::Response;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;

use crate::http::request::HyperRequest;

pub mod register_routes;
pub mod response;
mod routes;

pub struct StaticRouteOptions {
    pub context: Arc<Context>,
    pub request: HyperRequest,
}

pub type StaticRouteFnResult = BoxFuture<'static, Result<Response, HttpError>>;
pub type StaticRouteFn = fn(options: StaticRouteOptions) -> StaticRouteFnResult;
type StaticRoutes = HashMap<String, Box<StaticRouteFn>>;

pub struct Routing {
    pub static_get_routes: StaticRoutes,
}

impl Routing {
    pub fn new() -> Self {
        Self {
            static_get_routes: HashMap::new(),
        }
    }

    fn add_static_get_route(&mut self, path: &str, handler: StaticRouteFn) -> () {
        self.static_get_routes
            .insert(path.to_string(), Box::new(handler));
    }
}

pub trait Request {
    fn method(&self) -> &str;
}
