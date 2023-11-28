use std::collections::VecDeque;
use std::rc::Rc;

use enum_as_inner::EnumAsInner;
use lru::LruCache;

use diffbelt_protos::protos::transform::aggregate::{AggregateReduceInput, AggregateReduceItem};
use diffbelt_protos::{Serializer, WIPOffset};
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_util_no_std::buffers_pool::BuffersPool;
use diffbelt_util_no_std::temporary_collection::hash_set::TemporaryRefHashSet;

use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use crate::base::action::Action;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use crate::base::error::TransformError;
use crate::transform::{ActionInputHandlerActionsVec, TransformInputs};

pub struct AggregateTransform {
    pub(super) from_collection_name: Box<str>,
    pub(super) to_collection_name: Box<str>,
    pub(super) reader_name: Box<str>,
    pub(super) state: State,
    pub(super) action_input_handlers: TransformInputs<Self, HandlerContext>,
    pub(super) max_limits: Limits,
    pub(super) supports_accumulator_merge: bool,
    pub(super) free_map_eval_action_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_map_eval_input_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_target_info_action_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_reduce_eval_action_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_reduce_eval_input_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_serializer_reduce_input_items_buffers:
        BuffersPool<Vec<WIPOffset<AggregateReduceItem<'static>>>>,
}

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
    pub from_generation_id: EncodedGenerationIdJsonData,
    pub to_generation_id: EncodedGenerationIdJsonData,
    pub current_limits: Limits,
    pub target_keys: LruCache<Rc<[u8]>, TargetKeyData>,
    pub updated_target_keys_temp_set: TemporaryRefHashSet<[u8]>,
    pub reducing_chunk_id_counter: u64,
}

#[derive(Debug)]
pub struct TargetKeyCollectingChunk {
    pub accumulator_id: Option<AccumulatorId>,
    pub is_accumulator_pending: bool,
    pub reduce_input: Serializer<'static, AggregateReduceInput<'static>>,
    pub reduce_input_items: Vec<WIPOffset<AggregateReduceItem<'static>>>,
}

pub type TargetKeyReducingChunkId = u64;

#[derive(Debug)]
pub struct TargetKeyReducingChunk {
    pub chunk_id: TargetKeyReducingChunkId,
}

#[derive(Debug, EnumAsInner)]
pub enum TargetKeyChunk {
    Collecting(TargetKeyCollectingChunk),
    Reducing(TargetKeyReducingChunk),
}

#[derive(Debug)]
pub struct TargetKeyData {
    pub target_info_id: Option<TargetInfoId>,
    pub chunks: VecDeque<TargetKeyChunk>,
    pub is_target_info_pending: bool,
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
