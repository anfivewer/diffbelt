use serde::Deserialize;
use crate::code::Code;
use crate::transforms::TranformTargetKey;

#[derive(Debug, Deserialize)]
pub struct Percentiles {
    pub percentiles: Vec<String>,
    pub target_key: TranformTargetKey,
    pub intermediate: Code,
    pub empty_accumulator: Code,
    pub initial_accumulator: Code,
    pub reduce: Code,
    pub percentiles_data: Code,
    pub apply: Code,
}