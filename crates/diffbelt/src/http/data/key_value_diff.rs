use crate::common::{KeyValueDiff, OwnedCollectionValue};
use crate::http::data::encoded_value::{EncodedValueJsonData, EncodedValueJsonDataTrait};

use crate::http::data::encoded_key::{EncodedKeyJsonData, EncodedKeyJsonDataTrait};
pub use diffbelt_types::collection::diff::KeyValueDiffJsonData;

impl From<KeyValueDiff> for KeyValueDiffJsonData {
    fn from(kv: KeyValueDiff) -> Self {
        Self {
            key: EncodedKeyJsonData::encode(kv.key),
            from_value: opt_value_to_nullable_encoded_value(kv.from_value),
            intermediate_values: EncodedValueJsonData::encode_opt_vec(kv.intermediate_values),
            to_value: opt_value_to_nullable_encoded_value(kv.to_value),
        }
    }
}

pub trait KeyValueDiffJsonDataTrait: Sized {
    fn encode_vec(items: Vec<KeyValueDiff>) -> Vec<Self>;
}

impl KeyValueDiffJsonDataTrait for KeyValueDiffJsonData {
    fn encode_vec(items: Vec<KeyValueDiff>) -> Vec<Self> {
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
