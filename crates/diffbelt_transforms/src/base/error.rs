use diffbelt_protos::InvalidFlatbuffer;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_types::errors::IntoBytesError;
use diffbelt_util::errors::NoStdErrorWrap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("{0}")]
    Unspecified(String),
    #[error("{0:?}")]
    GenerationIdJsonDataIntoBytes(#[from] IntoBytesError<EncodedGenerationIdJsonData>),
    #[error("{0:?}")]
    EncodedKeyJsonDataIntoBytes(#[from] IntoBytesError<EncodedKeyJsonData>),
    #[error("{0:?}")]
    EncodedValueJsonDataIntoBytes(#[from] IntoBytesError<EncodedValueJsonData>),
    #[error(transparent)]
    InvalidFlatbuffer(#[from] NoStdErrorWrap<InvalidFlatbuffer>),
}
