use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use lru::LruCache;
use crate::base::error::TransformError;
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use crate::aggregate::limits::Limits;

#[derive(Debug)]
pub enum State {
    Uninitialized,
    AwaitingDiff,
    AwaitingGenerationStart {
        diff: DiffCollectionResponseJsonData,
    },
    Processing(ProcessingState),
    Invalid,
}

#[derive(Debug)]
pub struct ProcessingState {
    pub cursor_id: Option<Box<str>>,
    pub to_generation_id: EncodedGenerationIdJsonData,
    pub current_limits: Limits,
    pub target_keys: LruCache<Rc<[u8]>, TargetKeyData>,
}

#[derive(Debug)]
pub struct TargetKeyData {

}

impl State {
    pub fn expect_processing_mut(&mut self) -> Result<&mut ProcessingState, TransformError> {
        match self {
            State::Processing(state) => Ok(state),
            _ => Err(TransformError::Unspecified(format!(
                "expected processing state, got {self:?}"
            ))),
        }
    }
}
