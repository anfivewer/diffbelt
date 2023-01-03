use crate::common::{IsByteArray, KeyValue};
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueJsonData {
    key: String,
    key_encoding: Option<String>,

    value: String,
    value_encoding: Option<String>,
}

impl From<KeyValue> for KeyValueJsonData {
    fn from(kv: KeyValue) -> Self {
        let (key, key_encoding) =
            StrSerializationType::Utf8.serialize_with_priority(kv.key.get_byte_array());
        let (value, value_encoding) =
            StrSerializationType::Utf8.serialize_with_priority(kv.value.get_value());

        Self {
            key,
            key_encoding: key_encoding.to_optional_string(),
            value,
            value_encoding: value_encoding.to_optional_string(),
        }
    }
}
