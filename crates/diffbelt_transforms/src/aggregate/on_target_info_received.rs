use crate::aggregate::context::{HandlerContext, TargetRecordContext};
use crate::aggregate::on_map_received::on_target_info_available;
use crate::aggregate::AggregateTransform;
use crate::base::input::function_eval::AggregateTargetInfoEvalInput;
use crate::transform::{ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn on_target_info_received(
        &mut self,
        ctx: TargetRecordContext,
        input: AggregateTargetInfoEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        let TargetRecordContext {
            target_key: target_key_rc,
        } = ctx;
        let AggregateTargetInfoEvalInput {
            target_info_id,
            target_info_data_bytes,
        } = input;

        let target = state
            .target_keys
            .get_mut(&target_key_rc)
            .expect("target cannot be removed while there is pending get target record");

        assert!(
            target.is_target_info_pending,
            "there should be no multiple pending get target info for same target key"
        );
        assert!(
            target.target_info_id.is_none(),
            "if target info pending, there should be no target info id"
        );

        target.is_target_info_pending = false;
        target.target_info_id = Some(target_info_id);
        state.current_limits.target_data_bytes += target_info_data_bytes;

        assert_eq!(
            target.chunks.len(),
            1,
            "pending target info record can have only one chunk"
        );

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        on_target_info_available(
            target_key_rc,
            &mut actions,
            target,
            target_info_id,
            &mut state.current_limits,
            self.supports_accumulator_merge,
            &mut state.chunk_id_counter,
            &mut self.free_reduce_eval_action_buffers,
            &mut self.free_serializer_reduce_input_items_buffers,
        );

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
