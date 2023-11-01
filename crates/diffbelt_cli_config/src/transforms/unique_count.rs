use crate::transforms::wasm::WasmMethodDef;
use crate::transforms::TranformTargetKey;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UniqueCount {
    pub target_key: TranformTargetKey,
    pub intermediate: WasmMethodDef,
    pub empty_target: WasmMethodDef,
    pub apply: WasmMethodDef,
}
