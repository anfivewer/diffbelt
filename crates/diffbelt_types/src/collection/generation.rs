use crate::common::generation_id::EncodedGenerationIdJsonData;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGenerationRequestJsonData {
    pub generation_id: EncodedGenerationIdJsonData,
    pub abort_outdated: Option<bool>,
}
