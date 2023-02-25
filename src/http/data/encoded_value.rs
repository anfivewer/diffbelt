use crate::common::{IsByteArray, OwnedCollectionValue};

use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
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
    pub fn encode(value: OwnedCollectionValue) -> Self {
        let (value, encoding) =
            StrSerializationType::Utf8.serialize_with_priority(value.get_value());

        Self {
            value,
            encoding: encoding.to_optional_string(),
        }
    }

    pub fn encode_vec(items: Vec<OwnedCollectionValue>) -> Vec<Self> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            result.push(Self::encode(item));
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

            result.push(Some(Self::encode(value)));
        }

        result
    }

    pub fn into_collection_value(self) -> Result<OwnedCollectionValue, HttpError> {
        let encoding = StrSerializationType::from_opt_str(self.encoding)
            .map_err(|_| HttpError::Generic400("invalid encoding"))?;
        let bytes = encoding
            .deserialize(self.value)
            .map_err(|_| HttpError::Generic400("invalid serialization"))?;
        Ok(OwnedCollectionValue::new(&bytes))
    }

    pub fn decode_opt(
        value: Option<EncodedValueJsonData>,
    ) -> Result<Option<OwnedCollectionValue>, HttpError> {
        let Some(value) = value else {
            return Ok(None);
        };

        let value = value.into_collection_value()?;

        Ok(Some(value))
    }
}
