use crate::base::action::Action;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::error::TransformError;
use crate::base::input::Input;
use crate::map_filter::MapFilterTransform;
use crate::TransformRunResult;
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait Transform {
    fn run(&mut self, inputs: &mut Vec<Input>) -> Result<TransformRunResult, TransformError>;
    fn return_actions_vec(&mut self, buffer: Vec<Action>);

    fn return_target_info_action_buffer(&mut self, _buffer: Vec<u8>) {
        //
    }
    fn return_merge_accumulator_ids_vec(&mut self, _buffer: Vec<AccumulatorId>) {
        //
    }
}

#[enum_dispatch(Transform)]
pub enum TransformImpl {
    MapFilter(MapFilterTransform),
}
