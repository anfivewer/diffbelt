use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_types::errors::IntoBytesError;

#[derive(Debug)]
pub enum TransformError {
    Unspecified(String),
    GenerationIdJsonDataIntoBytes(IntoBytesError<EncodedGenerationIdJsonData>),
    EncodedKeyJsonDataIntoBytes(IntoBytesError<EncodedKeyJsonData>),
    EncodedValueJsonDataIntoBytes(IntoBytesError<EncodedValueJsonData>),
}

impl From<IntoBytesError<EncodedGenerationIdJsonData>> for TransformError {
    fn from(value: IntoBytesError<EncodedGenerationIdJsonData>) -> Self {
        Self::GenerationIdJsonDataIntoBytes(value)
    }
}

impl From<IntoBytesError<EncodedKeyJsonData>> for TransformError {
    fn from(value: IntoBytesError<EncodedKeyJsonData>) -> Self {
        Self::EncodedKeyJsonDataIntoBytes(value)
    }
}

impl From<IntoBytesError<EncodedValueJsonData>> for TransformError {
    fn from(value: IntoBytesError<EncodedValueJsonData>) -> Self {
        Self::EncodedValueJsonDataIntoBytes(value)
    }
}
