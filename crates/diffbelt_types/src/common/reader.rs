use serde::{Deserialize, Serialize};
use crate::common::generation_id::EncodedGenerationIdJsonData;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReaderJsonData {
    pub reader_name: String,
    pub generation_id: EncodedGenerationIdJsonData,
}
