use crate::common::{GenerationId, OwnedGenerationId};
use crate::http::errors::HttpError;
use diffbelt_protos::Vector;

pub fn flatbuffers_generation_id_to_owned_generation_id(
    generation_id: Option<Vector<'_, u8>>,
) -> Result<OwnedGenerationId, HttpError> {
    let generation_id = generation_id
        .ok_or_else(|| HttpError::GenericFlatbuffers400("empty generation id"))?
        .bytes();
    let generation_id = GenerationId::validate(generation_id)
        .map_err(|_| HttpError::GenericFlatbuffers400("invalid generation id"))?;
    Ok(generation_id.to_owned())
}
