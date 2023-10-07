use crate::value_encoding_into_bytes;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedKeyJsonData {
    pub value: String,
    pub encoding: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedValueJsonData {
    pub value: String,
    pub encoding: Option<String>,
}

value_encoding_into_bytes!(EncodedKeyJsonData);
value_encoding_into_bytes!(EncodedValueJsonData);
