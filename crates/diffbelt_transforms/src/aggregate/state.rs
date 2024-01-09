use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use enum_as_inner::EnumAsInner;
use lru::LruCache;

use diffbelt_protos::protos::transform::aggregate::{AggregateReduceInput, AggregateReduceItem};
use diffbelt_protos::{Serializer, WIPOffset};
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_util_no_std::buffers_pool::BuffersPool;
use diffbelt_util_no_std::temporary_collection::immutable::hash_set::TemporaryRefHashSet;
use diffbelt_util_no_std::temporary_collection::mutable::vec::TemporaryMutRefVec;
use diffbelt_util_no_std::temporary_collection::vec::{TemporaryVec, TempVecType};

use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use crate::base::error::TransformError;
use crate::transform::TransformInputs;

pub struct AggregateTransform {
    pub(super) from_collection_name: Box<str>,
    pub(super) to_collection_name: Box<str>,
    pub(super) reader_name: Box<str>,
    pub(super) state: State,
    pub(super) action_input_handlers: TransformInputs<Self, HandlerContext>,
    pub(super) max_limits: Limits,
    pub(super) supports_accumulator_merge: bool,
    pub(super) updated_target_keys_temp_set: TemporaryRefHashSet<[u8]>,
    pub(super) apply_target_keys_temp_vec: TemporaryVec<TargetKvTemp>,
    pub(super) free_map_eval_action_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_map_eval_input_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_target_info_action_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_reduce_eval_action_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_apply_eval_buffers: BuffersPool<Vec<u8>>,
    pub(super) free_target_keys_buffers: BuffersPool<Vec<Rc<[u8]>>>,
    pub(super) free_serializer_reduce_input_items_buffers:
        BuffersPool<Vec<WIPOffset<AggregateReduceItem<'static>>>>,
    pub(super) free_merge_accumulator_ids_vecs: BuffersPool<Vec<AccumulatorId>>,
}

pub struct TargetKvTemp;
impl TempVecType for TargetKvTemp {
    type Item<'a> = ((Rc<[u8]>, &'a mut Target));
}

#[derive(Debug)]
pub enum State {
    Uninitialized,
    AwaitingDiff,
    AwaitingGenerationStart {
        diff: DiffCollectionResponseJsonData,
    },
    Processing(ProcessingState),
    AwaitingCommitGeneration,
    Invalid,
}

#[derive(Debug)]
pub struct ProcessingState {
    pub cursor_id: Option<Box<str>>,
    pub from_generation_id: EncodedGenerationIdJsonData,
    pub to_generation_id: EncodedGenerationIdJsonData,
    pub current_limits: Limits,
    pub target_keys: LruCache<Rc<[u8]>, Target>,
    pub chunk_id_counter: u64,
    pub apply_puts: HashMap<Rc<[u8]>, Option<Box<[u8]>>>,
}

#[derive(Debug)]
pub struct TargetKeyCollectingChunk {
    pub accumulator_id: Option<AccumulatorId>,
    pub accumulator_data_bytes: u64,
    pub is_accumulator_pending: bool,
    pub is_reducing: bool,
    pub reduce_input: Serializer<'static, AggregateReduceInput<'static>>,
    pub reduce_input_items: Vec<WIPOffset<AggregateReduceItem<'static>>>,
}

pub type TargetKeyReducingChunkId = u64;
pub type TargetKeyMergingChunkId = u64;
pub type TargetKeyApplyId = u64;

#[derive(Debug)]
pub struct TargetKeyReducingChunk {
    pub chunk_id: TargetKeyReducingChunkId,
}

#[derive(Debug)]
pub struct TargetKeyReducedChunk {
    pub accumulator_id: AccumulatorId,
    pub accumulator_data_bytes: u64,
}

#[derive(Debug)]
pub struct TargetKeyMergingChunk {
    pub chunk_id: TargetKeyMergingChunkId,
}

#[derive(Debug, EnumAsInner)]
pub enum TargetKeyChunk {
    Tombstone,
    Collecting(TargetKeyCollectingChunk),
    Reducing(TargetKeyReducingChunk),
    Reduced(TargetKeyReducedChunk),
    Merging(TargetKeyMergingChunk),
}

#[derive(Debug, EnumAsInner)]
pub enum Target {
    Processing(TargetKeyData),
    Applying(TargetKeyApplying),
}

#[derive(Debug)]
pub struct TargetKeyData {
    pub target_info_id: Option<TargetInfoId>,
    pub target_info_data_bytes: u64,
    pub chunks: VecDeque<TargetKeyChunk>,
    pub is_target_info_pending: bool,
}

#[derive(Debug)]
pub struct TargetKeyApplying {
    // TODO: measure how frequently this vector is not zero
    /// Values that was received while this key was applied
    pub mapped_values: Vec<Option<Box<[u8]>>>,
    pub is_got_value: bool,
    pub is_putting: bool,
    pub target_kv_size: u64,
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
