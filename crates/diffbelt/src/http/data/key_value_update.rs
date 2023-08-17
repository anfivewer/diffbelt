use crate::common::{KeyValueUpdate, KeyValueUpdateNewOptions};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::json::serde::deserialize_strict_null;

use crate::http::data::encoded_key::EncodedKeyJsonData;
use crate::http::data::encoded_value::EncodedValueJsonData;
use serde::Deserialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueUpdateJsonData {
    key: EncodedKeyJsonData,
    if_not_present: Option<bool>,

    #[serde(deserialize_with = "deserialize_strict_null")]
    value: Option<EncodedValueJsonData>,
}

impl KeyValueUpdateJsonData {
    pub fn deserialize(self, decoder: &StringDecoder) -> Result<KeyValueUpdate, HttpError> {
        let key = self.key.decode(&decoder)?;
        let value = EncodedValueJsonData::decode_opt(self.value)?;

        let if_not_present = self.if_not_present.unwrap_or(false);

        Ok(KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key,
            value,
            if_not_present,
        }))
    }
}
