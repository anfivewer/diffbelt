use crate::common::KeyValue;

use crate::http::data::encoded_key::EncodedKeyJsonData;
use crate::http::data::encoded_value::EncodedValueJsonData;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueJsonData {
    key: EncodedKeyJsonData,
    value: EncodedValueJsonData,
}

impl From<KeyValue> for KeyValueJsonData {
    fn from(kv: KeyValue) -> Self {
        Self {
            key: EncodedKeyJsonData::encode(kv.key),
            value: EncodedValueJsonData::encode(kv.value),
        }
    }
}
