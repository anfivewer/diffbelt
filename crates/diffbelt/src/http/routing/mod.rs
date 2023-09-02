use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::response::Response;
use futures::future::BoxFuture;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Arc;

use crate::http::request::{HyperRequestWrapped, Request};

pub mod register_routes;
pub mod response;
mod routes;

pub struct StaticRouteOptions {
    pub context: Arc<Context>,
    pub request: HyperRequestWrapped,
}

pub type HttpHandlerResult = Result<Response, HttpError>;
pub type StaticRouteFnFutureResult = BoxFuture<'static, Result<Response, HttpError>>;
pub type StaticRouteFn = fn(options: StaticRouteOptions) -> StaticRouteFnFutureResult;

pub struct PatternRouteOptions<T> {
    pub context: Arc<Context>,
    pub request: HyperRequestWrapped,
    pub groups: T,
}

pub type PatternRouteFnResult = StaticRouteFnFutureResult;
pub type PatternRouteFn<T> = fn(options: PatternRouteOptions<T>) -> PatternRouteFnResult;

pub struct PatternRoute {
    pub path: Regex,
    pub handler: Box<
        dyn Fn(StaticRouteOptions, &Regex) -> Result<PatternRouteFnResult, StaticRouteOptions>
            + Sync
            + Send,
    >,
}

pub struct Routing {
    pub static_get_routes: HashMap<String, StaticRouteFn>,
    pub static_post_routes: HashMap<String, StaticRouteFn>,
    pub pattern_routes: Vec<PatternRoute>,
}

impl Routing {
    pub fn new() -> Self {
        Self {
            static_get_routes: HashMap::new(),
            static_post_routes: HashMap::new(),
            pattern_routes: Vec::new(),
        }
    }

    fn add_static_get_route(&mut self, path: &str, handler: StaticRouteFn) -> () {
        self.static_get_routes.insert(path.to_string(), handler);
    }
    fn add_static_post_route(&mut self, path: &str, handler: StaticRouteFn) -> () {
        self.static_post_routes.insert(path.to_string(), handler);
    }

    pub fn get_static_routes_by_method(
        &self,
        method: &str,
    ) -> Option<&HashMap<String, StaticRouteFn>> {
        match method {
            "GET" => Some(&self.static_get_routes),
            "POST" => Some(&self.static_post_routes),
            _ => None,
        }
    }

    fn add_pattern_route<T: 'static>(
        &mut self,
        path: Regex,
        make_groups: fn(Captures<'_>) -> T,
        route_handler: PatternRouteFn<T>,
    ) -> () {
        let handler = move |options: StaticRouteOptions,
                            regex: &Regex|
              -> Result<PatternRouteFnResult, StaticRouteOptions> {
            let path = options.request.get_path();
            let captures = regex.captures(path);

            let Some(captures) = captures else {
                return Err(options);
            };

            let groups = make_groups(captures);

            Ok(route_handler(PatternRouteOptions {
                context: options.context,
                request: options.request,
                groups,
            }))
        };

        self.pattern_routes.push(PatternRoute {
            path,
            handler: Box::new(handler),
        })
    }
}
