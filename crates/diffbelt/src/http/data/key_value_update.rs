use crate::common::{KeyValueUpdate, KeyValueUpdateNewOptions};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use diffbelt_types::common::key_value_update::KeyValueUpdateJsonData;

use crate::http::data::encoded_key::EncodedKeyJsonDataTrait;
use crate::http::data::encoded_value::{EncodedValueJsonData, EncodedValueJsonDataTrait};

pub trait KeyValueUpdateJsonDataTrait {
    fn deserialize(self, decoder: &StringDecoder) -> Result<KeyValueUpdate, HttpError>;
}

impl KeyValueUpdateJsonDataTrait for KeyValueUpdateJsonData {
    fn deserialize(self, decoder: &StringDecoder) -> Result<KeyValueUpdate, HttpError> {
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
