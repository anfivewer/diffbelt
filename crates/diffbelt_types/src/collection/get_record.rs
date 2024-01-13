use crate::common::generation_id::EncodedGenerationIdJsonData;
use crate::common::key_value::{EncodedKeyJsonData, KeyValueJsonData};
use crate::common::phantom_id::EncodedPhantomIdJsonData;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetRequestJsonData {
    pub key: EncodedKeyJsonData,
    pub generation_id: Option<EncodedGenerationIdJsonData>,
    pub phantom_id: Option<EncodedPhantomIdJsonData>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseJsonData {
    pub generation_id: EncodedGenerationIdJsonData,

    #[serialize_always]
    pub item: Option<KeyValueJsonData>,
}
