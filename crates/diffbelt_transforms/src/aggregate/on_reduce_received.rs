use crate::aggregate::context::{HandlerContext, ReducingContext};
use crate::aggregate::on_map_received::reduce_target_chunk;
use crate::aggregate::state::{TargetKeyChunk, TargetKeyCollectingChunk, TargetKeyReducedChunk};
use crate::aggregate::AggregateTransform;
use crate::base::error::TransformError;
use crate::base::input::function_eval::AggregateReduceEvalInput;
use crate::transform::{ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn on_reduce_received(
        &mut self,
        ctx: ReducingContext,
        input: AggregateReduceEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        let ReducingContext {
            target_key: target_key_rc,
            chunk_id,
            prev_accumulator_data_bytes,
            transferring_target_data_bytes,
        } = ctx;

        let AggregateReduceEvalInput {
            accumulator_id,
            accumulator_data_bytes: new_accumulator_data_bytes,
            action_input_buffer,
        } = input;

        self.free_reduce_eval_action_buffers
            .push(action_input_buffer);

        state.current_limits.target_data_bytes -= transferring_target_data_bytes;
        state.current_limits.target_data_bytes -= prev_accumulator_data_bytes;
        state.current_limits.target_data_bytes += new_accumulator_data_bytes;

        let target = state
            .target_keys
            .get_mut(&target_key_rc)
            .expect("target key should exist if reducing in progress")
            .as_processing_mut()
            .expect("target cannot be applied while there is pending reduce");

        let target_info_id = target
            .target_info_id
            .expect("target should be with target info id");

        let mut reduced_chunk = None;

        if self.supports_accumulator_merge {
            let chunks_iter = target.chunks.iter_mut().rev();

            for chunk in chunks_iter {
                if let TargetKeyChunk::Reducing(reducing) = chunk {
                    if reducing.chunk_id == chunk_id {
                        reduced_chunk = Some(chunk);
                        break;
                    }
                }
            }
        } else {
            assert_eq!(
                target.chunks.len(),
                1,
                "if merge not supported, there should be 1 chunk"
            );

            let chunk = target.chunks.front_mut().expect("just asserted");
            reduced_chunk = Some(chunk);
        }

        let Some(chunk) = reduced_chunk else {
            return Err(TransformError::Unspecified(
                "reducing record not found".to_string(),
            ));
        };

        state.current_limits.pending_reduces_count -= 1;

        if !self.supports_accumulator_merge {
            let TargetKeyChunk::Collecting(collecting) = chunk else {
                return Err(TransformError::Unspecified(
                    "on_reduce_received: not supports merge but no collection".to_string(),
                ));
            };

            let TargetKeyCollectingChunk {
                accumulator_id: chunk_accumulator_id,
                accumulator_data_bytes,
                is_accumulator_pending: _,
                is_reducing,
                reduce_input: _,
                reduce_input_items,
            } = collecting;

            *accumulator_data_bytes = new_accumulator_data_bytes;
            *chunk_accumulator_id = Some(accumulator_id);
            *is_reducing = false;

            let mut actions = self.action_input_handlers.take_action_input_actions_vec();

            if reduce_input_items.is_empty() {
                () = Self::try_apply(
                    &mut actions,
                    &mut state.chunk_id_counter,
                    &self.max_limits,
                    &mut state.current_limits,
                    &mut state.target_keys,
                    &mut self.apply_target_keys_temp_vec,
                    &mut self.free_apply_eval_buffers,
                );

                if actions.is_empty() {
                    self.action_input_handlers
                        .return_action_input_actions_vec(actions);
                    return Ok(ActionInputHandlerResult::Consumed);
                }

                return Ok(ActionInputHandlerResult::AddActions(actions));
            }

            reduce_target_chunk(
                target_key_rc,
                &mut actions,
                chunk,
                target_info_id,
                accumulator_id,
                &mut state.current_limits,
                self.supports_accumulator_merge,
                &mut state.chunk_id_counter,
                &mut self.free_reduce_eval_action_buffers,
                &mut self.free_serializer_reduce_input_items_buffers,
            );

            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        // Replace chunk
        *chunk = TargetKeyChunk::Reduced(TargetKeyReducedChunk {
            accumulator_id,
            accumulator_data_bytes: new_accumulator_data_bytes,
        });

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        Self::try_merge_chunks(
            &mut self.free_merge_accumulator_ids_vecs,
            &mut state.current_limits,
            &mut state.chunk_id_counter,
            &mut actions,
            target_key_rc,
            target_info_id,
            target,
        );
        () = Self::maybe_read_cursor(
            &mut actions,
            &self.max_limits,
            &mut state.current_limits,
            &self.from_collection_name,
            &mut state.cursor_id,
            None,
        );

        if actions.is_empty() {
            self.action_input_handlers
                .return_action_input_actions_vec(actions);
            return Ok(ActionInputHandlerResult::Consumed);
        }

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
