use diffbelt_types::collection::diff::KeyValueDiffJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use crate::base::error::TransformError;

pub enum State {
    Uninitialized,
    Initialization,
    AwaitingForGenerationStart(AwaitingForGenerationStartState),
    Processing(ProcessingState),
    Committing,
    Invalid,
}

impl State {
    pub fn as_mut_processing(&mut self) -> Result<&mut ProcessingState, TransformError> {
        let Self::Processing(state) = self else {
            return Err(TransformError::Unspecified(
                "State is not Processing".to_string(),
            ));
        };

        Ok(state)
    }

    pub fn into_processing(self) -> Result<ProcessingState, TransformError> {
        let Self::Processing(state) = self else {
            return Err(TransformError::Unspecified(
                "State is not Processing".to_string(),
            ));
        };

        Ok(state)
    }

    pub fn as_commiting(&self) -> Result<(), TransformError> {
        let Self::Committing = self else {
            return Err(TransformError::Unspecified(
                "State is not Commiting".to_string(),
            ));
        };

        Ok(())
    }
}

pub struct AwaitingForGenerationStartState {
    pub items: Vec<KeyValueDiffJsonData>,
    pub cursor_id: Option<Box<str>>,
    pub to_generation_id: EncodedGenerationIdJsonData,
}

pub struct ProcessingState {
    pub to_generation_id: EncodedGenerationIdJsonData,
    pub cursor_id: Option<Box<str>>,

    pub total_items: usize,
    pub total_chunks: usize,
}