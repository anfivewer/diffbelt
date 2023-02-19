use crate::common::{GenerationId, IsByteArray, OwnedGenerationId};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedGenerationIdFlatJsonData {
    generation_id: String,
    generation_id_encoding: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedOptionalGenerationIdFlatJsonData {
    generation_id: Option<String>,
    generation_id_encoding: Option<String>,
}

impl EncodedGenerationIdFlatJsonData {
    pub fn encode(generation_id: GenerationId<'_>, encoding: StrSerializationType) -> Self {
        let (generation_id, generation_id_encoding) =
            encoding.serialize_with_priority(generation_id.get_byte_array());

        Self {
            generation_id,
            generation_id_encoding: generation_id_encoding.to_optional_string(),
        }
    }

    pub fn decode(self, decoder: &StringDecoder) -> Result<OwnedGenerationId, HttpError> {
        decoder.decode_field_with_map(
            "generationId",
            self.generation_id,
            "generationIdEncoding",
            self.generation_id_encoding,
            |bytes| {
                OwnedGenerationId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid generationId, length should be <= 255",
                )))
            },
        )
    }
}

impl EncodedOptionalGenerationIdFlatJsonData {
    pub fn decode(self, decoder: &StringDecoder) -> Result<Option<OwnedGenerationId>, HttpError> {
        decoder.decode_opt_field_with_map(
            "generationId",
            self.generation_id,
            "generationIdEncoding",
            self.generation_id_encoding,
            |bytes| {
                OwnedGenerationId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid generationId, length should be <= 255",
                )))
            },
        )
    }

    pub fn decode_with_type(
        self,
        decoder: &StringDecoder,
    ) -> Result<(Option<OwnedGenerationId>, StrSerializationType), HttpError> {
        decoder.decode_opt_field_with_map_and_type(
            "generationId",
            self.generation_id,
            "generationIdEncoding",
            self.generation_id_encoding,
            |bytes| {
                OwnedGenerationId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid generationId, length should be <= 255",
                )))
            },
        )
    }
}
