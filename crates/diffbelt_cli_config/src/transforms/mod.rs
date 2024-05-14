use std::rc::Rc;

use serde::Deserialize;

use crate::transforms::aggregate::Aggregate;
use crate::transforms::percentiles::Percentiles;
use crate::transforms::unique_count::UniqueCount;
use crate::transforms::wasm::WasmMethodDef;

pub mod aggregate;
pub mod percentiles;
pub mod unique_count;
pub mod wasm;

#[derive(Debug, Deserialize)]
pub struct Transform {
    pub name: Option<Rc<str>>,
    pub source: Rc<str>,
    pub target: Rc<str>,
    pub intermediate: Option<Rc<str>>,
    pub reader_name: Option<Rc<str>>,
    pub map_filter: Option<WasmMethodDef>,
    pub aggregate: Option<Aggregate>,
    pub percentiles: Option<Percentiles>,
    pub unique_count: Option<UniqueCount>,
}

#[derive(Debug, Deserialize)]
pub struct TranformTargetKey {
    pub source: WasmMethodDef,
    pub intermediate: WasmMethodDef,
}
