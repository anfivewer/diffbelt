use crate::transforms::wasm::WasmMethodDef;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Aggregate {
    pub map: WasmMethodDef,
    pub initial_accumulator: WasmMethodDef,
    pub reduce: WasmMethodDef,
    pub merge_accumulators: WasmMethodDef,
    pub apply: WasmMethodDef,
    pub human_readable: Option<AggregateHumanReadable>,
}


#[derive(Debug, Deserialize)]
pub struct AggregateHumanReadable {
    pub wasm: String,
    pub mapped_key_from_bytes: Option<String>,
    pub mapped_value_from_bytes: Option<String>,
}