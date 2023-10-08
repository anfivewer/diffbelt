use crate::common::generation_id::EncodedGenerationIdJsonData;
use crate::common::reader::UpdateReaderJsonData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartGenerationRequestJsonData {
    pub generation_id: EncodedGenerationIdJsonData,
    pub abort_outdated: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitGenerationRequestJsonData {
    pub generation_id: EncodedGenerationIdJsonData,
    pub update_readers: Option<Vec<UpdateReaderJsonData>>,
}
