use diffbelt_wasm_binding::requests::RequestId;
use crate::update_ms::percentiles::reduce::KeysAround;
use crate::update_ms::percentiles::reduce::parsed_intermediate_key::ParsedIntermediateKey;

pub struct Percentile {
    pub percentile: f32,
    pub intermediate_key: Option<ParsedIntermediateKey>,
    pub key_pos: u32,
    pub keys_around: Option<KeysAround>,
    pub next_request_id: RequestId,
    pub next_request_is_forward: bool,
}
