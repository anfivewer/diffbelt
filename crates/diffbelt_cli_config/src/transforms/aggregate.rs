use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Aggregate {
    pub wasm: String,
    pub map: String,
    pub initial_accumulator: String,
    pub reduce: String,
    pub merge_accumulators: Option<String>,
    pub apply: String,
    pub human_readable: Option<AggregateHumanReadable>,
}

#[derive(Debug, Deserialize)]
pub struct AggregateHumanReadable {
    pub wasm: String,
    pub target_key_from_bytes: String,
    pub mapped_value_from_bytes: String,
    pub accumulator_from_bytes: String,
}
