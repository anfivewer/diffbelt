use crate::aggregate::limits::Limits;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use crate::base::error::TransformError;
use diffbelt_protos::protos::transform::aggregate::{AggregateReduceInput, AggregateReduceItem};
use diffbelt_protos::{Serializer, WIPOffset};
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_util_no_std::temporary_collection::vec::TemporaryRefVec;
use lru::LruCache;
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use diffbelt_util_no_std::temporary_collection::hash_set::TemporaryRefHashSet;

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
    pub updated_target_keys_temp_set: TemporaryRefHashSet<[u8]>,
}

#[derive(Debug)]
pub struct TargetKeyData {
    pub reduce_input: Serializer<'static, AggregateReduceInput<'static>>,
    pub reduce_input_items: Vec<WIPOffset<AggregateReduceItem<'static>>>,
    pub accumulator_and_target_info: Option<(AccumulatorId, TargetInfoId)>,
    pub is_accumulator_and_target_info_pending: bool,
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
