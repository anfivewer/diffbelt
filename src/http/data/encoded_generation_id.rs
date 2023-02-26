use crate::common::{GenerationId, IsByteArray, OwnedGenerationId};
use crate::http::errors::HttpError;

use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedGenerationIdJsonData {
    value: String,
    encoding: Option<String>,
}

impl EncodedGenerationIdJsonData {
    pub fn encode(generation_id: GenerationId<'_>, encoding: StrSerializationType) -> Self {
        let (value, generation_id_encoding) =
            encoding.serialize_with_priority(generation_id.get_byte_array());

        Self {
            value,
            encoding: generation_id_encoding.to_optional_string(),
        }
    }

    pub fn into_generation_id(self) -> Result<OwnedGenerationId, HttpError> {
        let encoding = StrSerializationType::from_opt_str(self.encoding)
            .map_err(|_| HttpError::Generic400("invalid encoding"))?;
        let bytes = encoding
            .deserialize(self.value)
            .map_err(|_| HttpError::Generic400("invalid serialization"))?;
        let generation_id = OwnedGenerationId::from_boxed_slice(bytes)
            .map_err(|_| HttpError::Generic400("invalid generationId size"))?;
        Ok(generation_id)
    }

    pub fn decode_opt(
        value: Option<EncodedGenerationIdJsonData>,
    ) -> Result<Option<OwnedGenerationId>, HttpError> {
        let Some(value) = value else {
            return Ok(None);
        };

        let value = value.into_generation_id()?;

        Ok(Some(value))
    }
}
