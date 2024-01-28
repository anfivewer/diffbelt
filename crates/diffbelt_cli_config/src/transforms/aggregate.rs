use crate::transforms::wasm::WasmMethodDef;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Aggregate {
    pub map: WasmMethodDef,
    pub target_info: WasmMethodDef,
    pub initial_accumulator: WasmMethodDef,
    pub reduce: WasmMethodDef,
    pub merge: WasmMethodDef,
    pub apply: WasmMethodDef,
}
