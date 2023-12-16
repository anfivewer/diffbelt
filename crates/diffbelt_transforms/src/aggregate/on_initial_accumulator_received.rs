use crate::aggregate::AggregateTransform;
use crate::aggregate::context::{HandlerContext, TargetRecordContext};
use crate::aggregate::on_map_received::reduce_target_chunk;
use crate::base::input::function_eval::AggregateInitialAccumulatorEvalInput;
use crate::transform::{ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn on_initial_accumulator_received(
        &mut self,
        ctx: TargetRecordContext,
        input: AggregateInitialAccumulatorEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        let TargetRecordContext {
            target_key: target_key_rc,
        } = ctx;
        let AggregateInitialAccumulatorEvalInput { accumulator_id } = input;

        let target = state
            .target_keys
            .get_mut(&target_key_rc)
            .expect("target cannot be removed while there is pending initial accumulator");

        assert!(
            !target.is_target_info_pending,
            "target info already should be present, not pending"
        );
        assert!(
            target.target_info_id.is_some(),
            "target info already should be present"
        );

        let target_info_id = target
            .target_info_id
            .expect("target info should be present");

        let last_chunk = target
            .chunks
            .back_mut()
            .expect("chunks should not be empty");
        let collecting_chunk = last_chunk
            .as_collecting_mut()
            .expect("last chunk should be Collecting");

        assert!(
            collecting_chunk.is_accumulator_pending,
            "got initial accumulator, but last chunk is not pending"
        );
        assert!(
            collecting_chunk.accumulator_id.is_none(),
            "got Some(accumulator_id) with pending"
        );

        collecting_chunk.is_accumulator_pending = false;
        collecting_chunk.accumulator_id = Some(accumulator_id);

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        reduce_target_chunk(
            target_key_rc,
            &mut actions,
            last_chunk,
            target_info_id,
            accumulator_id,
            self.supports_accumulator_merge,
            &mut state.chunk_id_counter,
            &mut self.free_reduce_eval_action_buffers,
            &mut self.free_serializer_reduce_input_items_buffers,
        );

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
