use crate::requests::client_impl::DiffbeltRequestsClientImpl;
use crate::requests::DiffbeltRequests;
use crate::wasm::cli_api::WasmCliApi;
use crate::wasm::engine::WasmEngine;
use diffbelt_http_client::client::DiffbeltClient;
use std::sync::Arc;

pub struct RunTestsOptions {
    pub with_unit: bool,
    pub with_integration: bool,
    pub wasm_engine: WasmEngine,
    pub client: Option<Arc<DiffbeltClient>>,
    pub cli_api: Option<WasmCliApi>,
}

pub struct RunTestsContext {
    pub wasm_engine: WasmEngine,
    pub client: Option<Arc<DiffbeltClient>>,
    pub requests: Arc<DiffbeltRequests>,
    pub requests_impl: DiffbeltRequestsClientImpl,
    pub cli_api: Option<WasmCliApi>,
}
