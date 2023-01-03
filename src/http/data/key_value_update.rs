use crate::common::{KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::json::serde::deserialize_strict_null;

use serde::Deserialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueUpdateJsonData {
    key: String,
    key_encoding: Option<String>,
    if_not_present: Option<bool>,

    #[serde(deserialize_with = "deserialize_strict_null")]
    value: Option<String>,
    value_encoding: Option<String>,
}

impl KeyValueUpdateJsonData {
    pub fn deserialize(self, decoder: &StringDecoder) -> Result<KeyValueUpdate, HttpError> {
        let key = decoder.decode_field_with_map(
            "key",
            self.key,
            "keyEncoding",
            self.key_encoding,
            |bytes| {
                OwnedCollectionKey::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid key, length should be <= 16777215",
                )))
            },
        )?;

        let value = decoder.decode_opt_field_with_map(
            "value",
            self.value,
            "valueEncoding",
            self.value_encoding,
            |bytes| Ok(OwnedCollectionValue::new(&bytes)),
        )?;

        let if_not_present = self.if_not_present.unwrap_or(false);

        Ok(KeyValueUpdate {
            key,
            value,
            if_not_present,
        })
    }
}
