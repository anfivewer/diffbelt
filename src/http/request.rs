use crate::http::routing::Request as RoutingRequest;
use hyper::{Body, Request};

pub struct HyperRequest {
    inner: Request<Body>,
}

impl HyperRequest {
    pub fn from(request: Request<Body>) -> Self {
        Self { inner: request }
    }
}

impl RoutingRequest for HyperRequest {
    fn method(&self) -> &str {
        self.inner.method().as_str()
    }
}
