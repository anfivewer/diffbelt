use crate::transforms::TranformTargetKey;
use serde::Deserialize;
use crate::transforms::wasm::WasmMethodDef;

#[derive(Debug, Deserialize)]
pub struct Percentiles {
    pub percentiles: Vec<String>,
    pub target_key: TranformTargetKey,
    pub intermediate: WasmMethodDef,
    pub empty_accumulator: WasmMethodDef,
    pub initial_accumulator: WasmMethodDef,
    pub reduce: WasmMethodDef,
    pub percentiles_data: WasmMethodDef,
    pub apply: WasmMethodDef,
}
