use diffbelt_protos::OwnedSerialized;
use diffbelt_protos::protos::transform::aggregate::AggregateMapMultiOutput;
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiOutput;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;

#[derive(Debug)]
pub struct FunctionEvalInput<T> {
    pub body: T,
}

#[derive(Debug)]
pub enum FunctionEvalInputBody {
    MapFilter(MapFilterEvalInput),
    AggregateMap(AggregateMapEvalInput),
    AggregateTargetInfo(AggregateTargetInfoEvalInput),
    AggregateInitialAccumulator(AggregateInitialAccumulatorEvalInput),
    AggregateReduce(AggregateReduceEvalInput),
    AggregateMerge(AggregateMergeEvalInput),
}

#[derive(Debug)]
pub struct MapFilterEvalRecord {
    pub pos: usize,
    pub len: usize,
}

#[derive(Debug)]
pub struct MapFilterEvalInput {
    pub input: OwnedSerialized<'static, MapFilterMultiOutput<'static>>,
    /// returned back `input` buffer from [`crate::base::action::function_eval::MapFilterEvalAction`]
    pub action_input_buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct AggregateMapEvalInput {
    pub input: OwnedSerialized<'static, AggregateMapMultiOutput<'static>>,
    /// returned back `input` buffer from [`crate::base::action::function_eval::AggregateMapEvalAction`]
    pub action_input_buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct AggregateTargetInfoEvalInput {
    pub target_info_id: TargetInfoId,
}

#[derive(Debug)]
pub struct AggregateInitialAccumulatorEvalInput {
    pub accumulator_id: AccumulatorId,
}

#[derive(Debug)]
pub struct AggregateReduceEvalInput {
    pub accumulator_id: AccumulatorId,
    /// returned back `input` buffer from [`crate::base::action::function_eval::AggregateReduceEvalAction`]
    pub action_input_buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct AggregateMergeEvalInput {
    pub accumulator_id: AccumulatorId,
}
