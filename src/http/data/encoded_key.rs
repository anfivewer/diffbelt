use crate::common::{IsByteArray, OwnedCollectionKey};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
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

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedKeyFlatJsonData {
    key: String,
    key_encoding: Option<String>,
}

impl EncodedKeyFlatJsonData {
    pub fn decode(self, decoder: &StringDecoder) -> Result<OwnedCollectionKey, HttpError> {
        decoder.decode_field_with_map("key", self.key, "keyEncoding", self.key_encoding, |bytes| {
            OwnedCollectionKey::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                "invalid key, length should be <= 16777215",
            )))
        })
    }
}
