use crate::common::OwnedCollectionValue;

use crate::http::errors::HttpError;

use crate::util::str_serialization::StrSerializationType;
pub use diffbelt_types::common::key_value::EncodedValueJsonData;
use serde::Deserialize;

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

pub trait EncodedValueJsonDataTrait: Sized {
    fn encode(value: OwnedCollectionValue) -> Self;
    fn encode_opt_vec(items: Vec<Option<OwnedCollectionValue>>) -> Vec<Option<Self>>;
    fn into_collection_value(self) -> Result<OwnedCollectionValue, HttpError>;
    fn decode_opt(
        value: Option<EncodedValueJsonData>,
    ) -> Result<Option<OwnedCollectionValue>, HttpError>;
}

impl EncodedValueJsonDataTrait for EncodedValueJsonData {
    fn encode(value: OwnedCollectionValue) -> Self {
        let (value, encoding) =
            StrSerializationType::Utf8.serialize_with_priority(value.get_value());

        Self {
            value,
            encoding: encoding.to_optional_string(),
        }
    }

    fn encode_opt_vec(items: Vec<Option<OwnedCollectionValue>>) -> Vec<Option<Self>> {
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

    fn into_collection_value(self) -> Result<OwnedCollectionValue, HttpError> {
        let encoding = StrSerializationType::from_opt_str(self.encoding)
            .map_err(|_| HttpError::Generic400("invalid encoding"))?;
        let bytes = encoding
            .deserialize(self.value)
            .map_err(|_| HttpError::Generic400("invalid serialization"))?;
        Ok(OwnedCollectionValue::new(&bytes))
    }

    fn decode_opt(
        value: Option<EncodedValueJsonData>,
    ) -> Result<Option<OwnedCollectionValue>, HttpError> {
        let Some(value) = value else {
            return Ok(None);
        };

        let value = value.into_collection_value()?;

        Ok(Some(value))
    }
}
