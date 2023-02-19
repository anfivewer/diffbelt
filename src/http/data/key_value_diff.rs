use crate::common::{IsByteArray, KeyValueDiff, OwnedCollectionValue};
use crate::http::data::encoded_value::EncodedValueJsonData;

use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueDiffJsonData {
    key: String,
    key_encoding: Option<String>,

    from_value: Option<Option<EncodedValueJsonData>>,
    intermediate_values: Vec<Option<EncodedValueJsonData>>,
    to_value: Option<Option<EncodedValueJsonData>>,
}

impl From<KeyValueDiff> for KeyValueDiffJsonData {
    fn from(kv: KeyValueDiff) -> Self {
        let (key, key_encoding) =
            StrSerializationType::Utf8.serialize_with_priority(kv.key.get_byte_array());

        Self {
            key,
            key_encoding: key_encoding.to_optional_string(),
            from_value: opt_value_to_nullable_encoded_value(kv.from_value),
            intermediate_values: EncodedValueJsonData::encode_opt_vec(kv.intermediate_values),
            to_value: opt_value_to_nullable_encoded_value(kv.to_value),
        }
    }
}

impl KeyValueDiffJsonData {
    pub fn encode_vec(items: Vec<KeyValueDiff>) -> Vec<Self> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            result.push(Self::from(item));
        }

        result
    }
}

fn opt_value_to_nullable_encoded_value(
    value: Option<OwnedCollectionValue>,
) -> Option<Option<EncodedValueJsonData>> {
    let Some(value) = value else {
        return Some(None);
    };

    Some(Some(EncodedValueJsonData::from(value)))
}
