use diffbelt_types::common::key_value::KeyValueJsonData;

use crate::common::KeyValue;
use crate::http::data::encoded_key::{EncodedKeyJsonData, EncodedKeyJsonDataTrait};
use crate::http::data::encoded_value::{EncodedValueJsonData, EncodedValueJsonDataTrait};

impl From<KeyValue> for KeyValueJsonData {
    fn from(kv: KeyValue) -> Self {
        Self {
            key: EncodedKeyJsonData::encode(kv.key),
            value: EncodedValueJsonData::encode(kv.value),
        }
    }
}
