use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CollectionHumanReadableConfig {
    pub wasm: String,
    pub key_to_bytes: String,
    pub bytes_to_key: String,
    pub value_to_bytes: String,
    pub bytes_to_value: String,
}