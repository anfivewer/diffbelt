use crate::aggregate::context::{HandlerContext, MergingContext};
use crate::aggregate::state::{TargetKeyChunk, TargetKeyReducedChunk};
use crate::aggregate::AggregateTransform;
use crate::base::input::function_eval::AggregateMergeEvalInput;
use crate::transform::{ActionInputHandlerResult, HandlerResult};
use std::mem;

impl AggregateTransform {
    pub fn on_merge_received(
        &mut self,
        ctx: MergingContext,
        input: AggregateMergeEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;
        let MergingContext {
            target_key_rc,
            chunk_id,
        } = ctx;
        let AggregateMergeEvalInput { accumulator_id } = input;

        let target = state
            .target_keys
            .get_mut(&target_key_rc)
            .expect("target key should exist if merging in progress");

        let target_info_id = target
            .target_info_id
            .expect("target should be with target info id");

        let chunk = target
            .chunks
            .iter_mut()
            .find(|chunk| {
                chunk
                    .as_merging()
                    .map(|chunk| chunk.chunk_id == chunk_id)
                    .unwrap_or(false)
            })
            .expect("chunk cannot disappear if merging in progress");

        *chunk = TargetKeyChunk::Reduced(TargetKeyReducedChunk { accumulator_id });

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        Self::try_merge_chunks(
            &mut self.free_merge_accumulator_ids_vecs,
            &mut state.chunk_id_counter,
            &mut actions,
            target_key_rc,
            target_info_id,
            target,
        );

        let need_try_apply = actions.is_empty();

        () = Self::maybe_read_cursor(
            &mut actions,
            &self.max_limits,
            &state.current_limits,
            &self.from_collection_name,
            &mut state.cursor_id,
            None,
        );

        if need_try_apply {
            () = Self::try_apply(&mut actions);
        }

        if actions.is_empty() {
            self.action_input_handlers
                .return_action_input_actions_vec(actions);

            return Ok(ActionInputHandlerResult::Consumed);
        }

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}