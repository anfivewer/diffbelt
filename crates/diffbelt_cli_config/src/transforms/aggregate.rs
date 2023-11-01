use crate::transforms::wasm::WasmMethodDef;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Aggregate {
    pub key: WasmMethodDef,
    pub map_filter: WasmMethodDef,
    pub empty_accumulator: WasmMethodDef,
    pub initial_accumulator: WasmMethodDef,
    pub reduce: WasmMethodDef,
    pub merge_accumulators: WasmMethodDef,
    pub apply: WasmMethodDef,
}
