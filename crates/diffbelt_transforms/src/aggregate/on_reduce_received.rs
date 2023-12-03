use crate::aggregate::context::{HandlerContext, ReducingContext, TargetRecordContext};
use crate::aggregate::on_map_received::reduce_target_chunk;
use crate::aggregate::state::{TargetKeyChunk, TargetKeyCollectingChunk, TargetKeyReducedChunk};
use crate::aggregate::AggregateTransform;
use crate::base::error::TransformError;
use crate::base::input::function_eval::{
    AggregateInitialAccumulatorEvalInput, AggregateReduceEvalInput,
};
use crate::transform::{ActionInputHandlerResult, HandlerResult};
use diffbelt_protos::Serializer;
use std::mem;

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
        } = ctx;

        let AggregateReduceEvalInput {
            accumulator_id,
            action_input_buffer,
        } = input;

        self.free_reduce_eval_action_buffers
            .push(action_input_buffer);

        let target = state
            .target_keys
            .get_mut(&target_key_rc)
            .expect("target key should exist if reducing in progress");

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

        if !self.supports_accumulator_merge {
            let TargetKeyChunk::Collecting(collecting) = chunk else {
                return Err(TransformError::Unspecified(
                    "on_reduce_received: not supports merge but no collection".to_string(),
                ));
            };

            let TargetKeyCollectingChunk {
                accumulator_id: chunk_accumulator_id,
                is_accumulator_pending: _,
                is_reducing,
                reduce_input: _,
                reduce_input_items,
            } = collecting;

            *chunk_accumulator_id = Some(accumulator_id);
            *is_reducing = false;

            if reduce_input_items.is_empty() {
                return Ok(ActionInputHandlerResult::Consumed);
            }

            let mut actions = self.action_input_handlers.take_action_input_actions_vec();

            reduce_target_chunk(
                target_key_rc,
                &mut actions,
                chunk,
                target_info_id,
                accumulator_id,
                self.supports_accumulator_merge,
                &mut state.reducing_chunk_id_counter,
                &mut self.free_reduce_eval_action_buffers,
                &mut self.free_serializer_reduce_input_items_buffers,
            );

            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        // Replace chunk
        let mut new_chunk = TargetKeyChunk::Reduced(TargetKeyReducedChunk { accumulator_id });
        mem::swap(chunk, &mut new_chunk);
        let old_chunk = chunk;

        todo!("try merge chunks")
    }
}
