use diffbelt_types::collection::get_record::GetRequestJsonData;
use diffbelt_types::common::key_value::EncodedKeyJsonData;
use std::borrow::Cow;
use std::mem;

use crate::aggregate::context::{
    HandlerContext, MergingContext, ReducingContext, TargetRecordContext,
};
use crate::aggregate::on_map_received::reduce_target_chunk;
use crate::aggregate::state::{
    TargetKeyChunk, TargetKeyCollectingChunk, TargetKeyMergingChunk, TargetKeyReducedChunk,
};
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::{
    AggregateMergeEvalAction, FunctionEvalAction, MapFilterEvalAction,
};
use crate::base::action::ActionType;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::error::TransformError;
use crate::base::input::function_eval::AggregateReduceEvalInput;
use crate::input_handler;
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
                &mut state.chunk_id_counter,
                &mut self.free_reduce_eval_action_buffers,
                &mut self.free_serializer_reduce_input_items_buffers,
            );

            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        // Replace chunk
        let mut new_chunk = TargetKeyChunk::Reduced(TargetKeyReducedChunk { accumulator_id });
        mem::swap(chunk, &mut new_chunk);

        // Try merge chunks
        let mut chunks_iter = target.chunks.iter_mut();
        let mut prev_reduced_chunk = chunks_iter.next().and_then(|chunk| {
            if chunk.is_reduced() {
                Some(chunk)
            } else {
                None
            }
        });

        let mut chunk_id = None;
        let mut accumulator_ids = self.free_merge_accumulator_ids_vecs.take();

        for chunk in chunks_iter {
            if prev_reduced_chunk.is_none() {
                // Find first reduced chunk
                if chunk.is_reduced() {
                    prev_reduced_chunk = Some(chunk);
                    continue;
                }

                continue;
            }

            // Collect accumulators to merge
            if let Some(reduced) = chunk.as_reduced() {
                if accumulator_ids.is_empty() {
                    accumulator_ids.push(
                        prev_reduced_chunk
                            .as_ref()
                            .expect("already checked")
                            .as_reduced()
                            .expect("already checked")
                            .accumulator_id,
                    );

                    state.chunk_id_counter += 1;
                    let new_chunk_id = state.chunk_id_counter;

                    chunk_id = Some(new_chunk_id);

                    prev_reduced_chunk = prev_reduced_chunk.map(|prev_reduced_chunk| {
                        let mut new_chunk = TargetKeyChunk::Merging(TargetKeyMergingChunk {
                            chunk_id: new_chunk_id,
                        });
                        mem::swap(prev_reduced_chunk, &mut new_chunk);

                        prev_reduced_chunk
                    });
                }

                accumulator_ids.push(reduced.accumulator_id);

                mem::swap(chunk, &mut TargetKeyChunk::Tombstone);
                continue;
            }

            // Continue to search first reduced chunk
            if accumulator_ids.is_empty() {
                prev_reduced_chunk = None;
                continue;
            }

            // Merge maybe found chain
            break;
        }

        let Some(chunk_id) = chunk_id else {
            return Ok(ActionInputHandlerResult::Consumed);
        };

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::AggregateMerge(
                AggregateMergeEvalAction {
                    target_info: target_info_id,
                    input: accumulator_ids,
                },
            )),
            HandlerContext::Merging(MergingContext {
                target_key: target_key_rc,
                chunk_id,
            }),
            input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                let ctx = ctx.into_merging().expect("should be MergingContext");
                todo!()
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
