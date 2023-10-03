use diffbelt_types::common::generation_id::IntoBytesError;

#[derive(Debug)]
pub enum TransformError {
    Unspecified(String),
    GenerationIdJsonDataIntoBytes(IntoBytesError),
}

impl From<IntoBytesError> for TransformError {
    fn from(value: IntoBytesError) -> Self {
        Self::GenerationIdJsonDataIntoBytes(value)
    }
}
