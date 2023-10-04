use crate::common::{IsByteArray, OwnedCollectionKey};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
pub use diffbelt_types::common::key_value::EncodedKeyJsonData;

pub trait EncodedKeyJsonDataTrait: Sized {
    fn encode(value: OwnedCollectionKey) -> Self;
    fn encode_vec(items: Vec<OwnedCollectionKey>) -> Vec<Self>;
    fn decode(self, decoder: &StringDecoder) -> Result<OwnedCollectionKey, HttpError>;
}

impl EncodedKeyJsonDataTrait for EncodedKeyJsonData {
    fn encode(value: OwnedCollectionKey) -> Self {
        let (value, encoding) =
            StrSerializationType::Utf8.serialize_with_priority(value.get_byte_array());

        Self {
            value,
            encoding: encoding.to_optional_string(),
        }
    }

    fn encode_vec(items: Vec<OwnedCollectionKey>) -> Vec<Self> {
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

    fn decode(self, decoder: &StringDecoder) -> Result<OwnedCollectionKey, HttpError> {
        decoder.decode_field_with_map("value", self.value, "encoding", self.encoding, |bytes| {
            OwnedCollectionKey::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                "invalid key, length should be <= 16777215",
            )))
        })
    }
}
