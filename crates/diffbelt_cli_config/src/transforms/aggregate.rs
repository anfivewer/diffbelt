use crate::code::Code;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Aggregate {
    pub key: Code,
    pub map_filter: Code,
    pub empty_accumulator: Code,
    pub initial_accumulator: Code,
    pub reduce: Code,
    pub merge_accumulators: Code,
    pub apply: Code,
}
