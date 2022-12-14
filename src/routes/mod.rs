use crate::context::Context;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod root;

pub struct StaticRouteOptions {
    pub context: Arc<RwLock<Context>>,
}

pub struct BaseResponse {
    pub status: u16,
}

impl Default for BaseResponse {
    fn default() -> Self {
        Self { status: 200 }
    }
}

pub struct StringResponse {
    pub base: BaseResponse,
    pub str: String,
}

pub enum Response {
    String(StringResponse),
}

pub type StaticRouteFnResult = BoxFuture<'static, Response>;
pub type StaticRouteFn = fn(options: StaticRouteOptions) -> StaticRouteFnResult;
type StaticRoutes = HashMap<String, Arc<StaticRouteFn>>;

pub struct Routing {
    pub static_get_routes: StaticRoutes,
}

impl Routing {
    pub fn add_static_get_route(&mut self, path: &str, handler: StaticRouteFn) -> () {
        self.static_get_routes
            .insert(path.to_string(), Arc::new(handler));
    }
}

pub fn new_routing() -> Routing {
    return Routing {
        static_get_routes: HashMap::new(),
    };
}
