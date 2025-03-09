use crate::requests::DiffbeltRequests;
use crate::wasm::engine::WasmEngine;
use diffbelt_http_client::client::DiffbeltClient;
use std::sync::Arc;
use crate::requests::client_impl::DiffbeltRequestsClientImpl;

pub struct RunTestsOptions {
    pub with_unit: bool,
    pub with_integration: bool,
    pub client: Option<Arc<DiffbeltClient>>,
}

impl Default for RunTestsOptions {
    fn default() -> Self {
        Self {
            with_unit: true,
            with_integration: true,
            client: None,
        }
    }
}

pub struct RunTestsContext {
    pub wasm_engine: WasmEngine,
    pub requests: Arc<DiffbeltRequests>,
    pub requests_impl: DiffbeltRequestsClientImpl,
}
