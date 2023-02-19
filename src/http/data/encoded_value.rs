use crate::common::{IsByteArray, OwnedCollectionValue};

use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedValueJsonData {
    value: String,
    encoding: Option<String>,
}

impl From<OwnedCollectionValue> for EncodedValueJsonData {
    fn from(value: OwnedCollectionValue) -> Self {
        let (value, encoding) =
            StrSerializationType::Utf8.serialize_with_priority(value.get_value());

        Self {
            value,
            encoding: encoding.to_optional_string(),
        }
    }
}

impl EncodedValueJsonData {
    pub fn encode_vec(items: Vec<OwnedCollectionValue>) -> Vec<Self> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let (value, encoding) =
                StrSerializationType::Utf8.serialize_with_priority(item.get_byte_array());

            result.push(Self {
                value,
                encoding: encoding.to_optional_string(),
            });
        }

        result
    }

    pub fn encode_opt_vec(items: Vec<Option<OwnedCollectionValue>>) -> Vec<Option<Self>> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let Some(value) = item else {
                result.push(None);
                continue;
            };

            let (value, encoding) =
                StrSerializationType::Utf8.serialize_with_priority(value.get_byte_array());

            result.push(Some(Self {
                value,
                encoding: encoding.to_optional_string(),
            }));
        }

        result
    }
}
