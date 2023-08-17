use crate::common::{GenerationId, IsByteArray, OwnedGenerationId};
use crate::http::errors::HttpError;

use crate::util::str_serialization::StrSerializationType;

pub use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;

pub fn encoded_generation_id_data_encode(
    generation_id: GenerationId<'_>,
    encoding: StrSerializationType,
) -> EncodedGenerationIdJsonData {
    let (value, generation_id_encoding) =
        encoding.serialize_with_priority(generation_id.get_byte_array());

    EncodedGenerationIdJsonData {
        value,
        encoding: generation_id_encoding.to_optional_string(),
    }
}

pub fn encoded_generation_id_data_into_generation_id(
    data: EncodedGenerationIdJsonData,
) -> Result<OwnedGenerationId, HttpError> {
    let encoding = StrSerializationType::from_opt_str(data.encoding)
        .map_err(|_| HttpError::Generic400("invalid encoding"))?;
    let bytes = encoding
        .deserialize(data.value)
        .map_err(|_| HttpError::Generic400("invalid serialization"))?;
    let generation_id = OwnedGenerationId::from_boxed_slice(bytes)
        .map_err(|_| HttpError::Generic400("invalid generationId size"))?;
    Ok(generation_id)
}

pub fn encoded_generation_id_data_decode_opt(
    value: Option<EncodedGenerationIdJsonData>,
) -> Result<Option<OwnedGenerationId>, HttpError> {
    let Some(value) = value else {
            return Ok(None);
        };

    let value = encoded_generation_id_data_into_generation_id(value)?;

    Ok(Some(value))
}
