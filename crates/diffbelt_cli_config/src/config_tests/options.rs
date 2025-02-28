use crate::wasm::engine::WasmEngine;
use diffbelt_http_client::client::DiffbeltClient;

pub struct RunTestsOptions<'a> {
    pub with_unit: bool,
    pub with_integration: bool,
    pub client: Option<&'a DiffbeltClient>,
}

impl Default for RunTestsOptions<'_> {
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
}
