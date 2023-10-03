use crate::common::generation_id::EncodedGenerationIdJsonData;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCollectionRequestJsonData {
    pub collection_id: String,
    pub with_generation_id: Option<bool>,
    pub with_next_generation_id: Option<bool>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCollectionResponseJsonData {
    pub is_manual: bool,
    pub generation_id: Option<EncodedGenerationIdJsonData>,
    pub next_generation_id: Option<Option<EncodedGenerationIdJsonData>>,
}
