use crate::common::{GenerationId, OwnedGenerationId, OwnedPhantomId, PhantomId};
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

pub fn flatbuffers_generation_id_to_owned_generation_id_opt(
    generation_id: Option<Vector<'_, u8>>,
) -> Result<Option<OwnedGenerationId>, HttpError> {
    let Some(generation_id) = generation_id else {
        return Ok(None);
    };
    let generation_id = GenerationId::validate(generation_id.bytes())
        .map_err(|_| HttpError::GenericFlatbuffers400("invalid generation id"))?;
    Ok(Some(generation_id.to_owned()))
}

pub fn flatbuffers_phantom_id_to_owned_phantom_id_opt(
    phantom_id: Option<Vector<'_, u8>>,
) -> Result<Option<OwnedPhantomId>, HttpError> {
    let Some(phantom_id) = phantom_id else {
        return Ok(None);
    };
    let phantom_id = PhantomId::new(phantom_id.bytes())
        .ok_or_else(|| HttpError::GenericFlatbuffers400("invalid phantom id"))?;
    Ok(Some(phantom_id.to_owned()))
}
