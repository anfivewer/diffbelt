use crate::common::generation_id::EncodedGenerationIdJsonData;
use serde::{Deserialize, Serialize};
use crate::common::reader::UpdateReaderJsonData;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGenerationRequestJsonData {
    pub generation_id: EncodedGenerationIdJsonData,
    pub abort_outdated: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitGenerationRequestJsonData {
    pub generation_id: EncodedGenerationIdJsonData,
    pub update_readers: Option<Vec<UpdateReaderJsonData>>,
}
