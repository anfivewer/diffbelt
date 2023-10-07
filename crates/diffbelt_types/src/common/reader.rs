use serde::{Deserialize, Serialize};
use crate::common::generation_id::EncodedGenerationIdJsonData;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReaderJsonData {
    pub reader_name: String,
    pub generation_id: EncodedGenerationIdJsonData,
}
