use serde::{Serialize, Deserialize};
use serde_with::skip_serializing_none;
use crate::common::generation_id::EncodedGenerationIdJsonData;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCollectionResponseJsonData {
    pub is_manual: bool,
    pub generation_id: Option<EncodedGenerationIdJsonData>,
    pub next_generation_id: Option<Option<EncodedGenerationIdJsonData>>,
}
