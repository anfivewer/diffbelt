use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateReduceInput, AggregateTargetInfo,
};
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput;
use diffbelt_protos::OwnedSerialized;

#[derive(Debug)]
pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
    AggregateMap(AggregateMapEvalAction),
    AggregateTargetInfo(AggregateTargetInfoEvalAction),
    AggregateInitialAccumulator(AggregateInitialAccumulatorEvalAction),
    AggregateReduce(AggregateReduceEvalAction),
    AggregateMerge(AggregateMergeEvalAction),
}

#[derive(Debug)]
pub struct MapFilterEvalAction {
    pub input: OwnedSerialized<'static, MapFilterMultiInput<'static>>,
    /// returned back `input` buffer from [`crate::base::input::function_eval::MapFilterEvalInput`]
    pub output_buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct AggregateMapEvalAction {
    pub input: OwnedSerialized<'static, AggregateMapMultiInput<'static>>,
    /// returned back `input` buffer from [`crate::base::input::function_eval::AggregateMapEvalInput`]
    pub output_buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct AggregateTargetInfoEvalAction {
    pub target_info: OwnedSerialized<'static, AggregateTargetInfo<'static>>,
}

#[derive(Debug)]
pub struct AggregateInitialAccumulatorEvalAction {
    pub target_info: TargetInfoId,
}

#[derive(Debug)]
pub struct AggregateReduceEvalAction {
    pub accumulator: AccumulatorId,
    pub target_info: TargetInfoId,
    pub input: OwnedSerialized<'static, AggregateReduceInput<'static>>,
}

#[derive(Debug)]
pub struct AggregateMergeEvalAction {
    pub target_info: TargetInfoId,
    pub accumulator_ids: Vec<AccumulatorId>,
}

#[derive(Debug)]
pub struct AggregateApplyEvalAction {
    pub target_info: TargetInfoId,
    pub accumulator: AccumulatorId,
    pub input: OwnedSerialized<'static, AggregateApplyOutput<'static>>,
    /// returned back `input` buffer from [`crate::base::input::function_eval::AggregateApplyEvalInput`]
    pub output_buffer: Vec<u8>,
}
