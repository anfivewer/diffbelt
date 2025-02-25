use enum_as_inner::EnumAsInner;

use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use diffbelt_protos::protos::impls::{
    AggregateMapMultiInputProto, AggregateReduceInputProto, AggregateTargetInfoProto,
    MapFilterMultiInputProto,
};
use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiInput, AggregateReduceInput, AggregateTargetInfo,
};
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput;
use diffbelt_protos::OwnedSerialized;

#[derive(Debug, EnumAsInner)]
pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
    AggregateMap(AggregateMapEvalAction),
    AggregateTargetInfo(AggregateTargetInfoEvalAction),
    AggregateInitialAccumulator(AggregateInitialAccumulatorEvalAction),
    AggregateReduce(AggregateReduceEvalAction),
    AggregateMerge(AggregateMergeEvalAction),
    AggregateApply(AggregateApplyEvalAction),
}

#[derive(Debug)]
pub struct MapFilterEvalAction {
    pub input: OwnedSerialized<MapFilterMultiInputProto>,
    /// returned back `input` buffer from [`crate::base::input::function_eval::MapFilterEvalInput`]
    pub output_buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct AggregateMapEvalAction {
    pub input: OwnedSerialized<AggregateMapMultiInputProto>,
}

#[derive(Debug)]
pub struct AggregateTargetInfoEvalAction {
    pub target_info: OwnedSerialized<AggregateTargetInfoProto>,
}

#[derive(Debug)]
pub struct AggregateInitialAccumulatorEvalAction {
    pub target_info: TargetInfoId,
}

#[derive(Debug)]
pub struct AggregateReduceEvalAction {
    pub accumulator: AccumulatorId,
    pub input: OwnedSerialized<AggregateReduceInputProto>,
}

#[derive(Debug)]
pub struct AggregateMergeEvalAction {
    pub accumulator_ids: Vec<AccumulatorId>,
}

#[derive(Debug)]
pub struct AggregateApplyEvalAction {
    pub accumulator: AccumulatorId,
}
