use crate::value_encoding_into_bytes;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EncodedGenerationIdJsonData {
    pub value: String,
    pub encoding: Option<String>,
}

impl EncodedGenerationIdJsonData {
    pub fn new_str(value: String) -> Self {
        Self {
            value,
            encoding: None,
        }
    }
}

value_encoding_into_bytes!(EncodedGenerationIdJsonData);
