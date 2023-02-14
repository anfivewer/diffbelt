use crate::common::{IsByteArray, OwnedCollectionKey};
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedKeyJsonData {
    key: String,
    encoding: Option<String>,
}

impl EncodedKeyJsonData {
    pub fn encode_vec(items: Vec<OwnedCollectionKey>) -> Vec<Self> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let (key, encoding) =
                StrSerializationType::Utf8.serialize_with_priority(item.get_byte_array());

            result.push(EncodedKeyJsonData {
                key,
                encoding: encoding.to_optional_string(),
            });
        }

        result
    }
}
