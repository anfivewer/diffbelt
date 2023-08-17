use crate::common::{IsByteArray, OwnedCollectionKey};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedKeyJsonData {
    value: String,
    encoding: Option<String>,
}

impl EncodedKeyJsonData {
    pub fn encode(value: OwnedCollectionKey) -> Self {
        let (value, encoding) =
            StrSerializationType::Utf8.serialize_with_priority(value.get_byte_array());

        Self {
            value,
            encoding: encoding.to_optional_string(),
        }
    }

    pub fn encode_vec(items: Vec<OwnedCollectionKey>) -> Vec<Self> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let (value, encoding) =
                StrSerializationType::Utf8.serialize_with_priority(item.get_byte_array());

            result.push(EncodedKeyJsonData {
                value,
                encoding: encoding.to_optional_string(),
            });
        }

        result
    }

    pub fn decode(self, decoder: &StringDecoder) -> Result<OwnedCollectionKey, HttpError> {
        decoder.decode_field_with_map("value", self.value, "encoding", self.encoding, |bytes| {
            OwnedCollectionKey::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                "invalid key, length should be <= 16777215",
            )))
        })
    }
}
