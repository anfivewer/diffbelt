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
    pub bytes_to_target_key: String,
    pub bytes_to_mapped_value: String,
    pub bytes_to_accumulator: String,
}
