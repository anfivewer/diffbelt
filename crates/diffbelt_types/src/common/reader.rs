use crate::common::generation_id::EncodedGenerationIdJsonData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReaderJsonData {
    pub reader_name: String,
    pub generation_id: EncodedGenerationIdJsonData,
}
