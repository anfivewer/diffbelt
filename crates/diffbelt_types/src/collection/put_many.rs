use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use crate::common::generation_id::{EncodedGenerationIdJsonData};
use crate::common::key_value_update::KeyValueUpdateJsonData;
use crate::common::phantom_id::EncodedPhantomIdJsonData;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PutManyRequestJsonData {
    pub items: Vec<KeyValueUpdateJsonData>,
    pub generation_id: Option<EncodedGenerationIdJsonData>,
    pub phantom_id: Option<EncodedPhantomIdJsonData>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutManyResponseJsonData {
    pub generation_id: EncodedGenerationIdJsonData,
}