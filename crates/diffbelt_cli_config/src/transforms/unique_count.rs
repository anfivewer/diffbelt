use crate::transforms::TranformTargetKey;
use serde::Deserialize;
use crate::transforms::wasm::WasmMethodDef;

#[derive(Debug, Deserialize)]
pub struct UniqueCount {
    pub target_key: TranformTargetKey,
    pub intermediate: WasmMethodDef,
    pub empty_target: WasmMethodDef,
    pub apply: WasmMethodDef,
}
