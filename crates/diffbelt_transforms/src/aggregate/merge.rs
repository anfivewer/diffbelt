use std::collections::VecDeque;
use std::rc::Rc;

use diffbelt_util_no_std::buffers_pool::BuffersPool;

use crate::aggregate::context::{HandlerContext, MergingContext};
use crate::aggregate::state::{
    ProcessingState, TargetKeyChunk, TargetKeyData, TargetKeyMergingChunk,
};
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateMergeEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use crate::base::input::function_eval::FunctionEvalInput;
use crate::input_handler;
use crate::transform::ActionInputHandlerActionsVec;

impl AggregateTransform {
    pub fn try_merge_chunks(
        free_merge_accumulator_ids_vecs: &mut BuffersPool<Vec<AccumulatorId>>,
        chunk_id_counter: &mut u64,
        actions: &mut ActionInputHandlerActionsVec<Self, HandlerContext>,
        target_key_rc: Rc<[u8]>,
        target_info_id: TargetInfoId,
        target: &mut TargetKeyData,
    ) {
        let mut chunks_iter = target.chunks.iter_mut();
        let mut prev_reduced_chunk = chunks_iter.next().and_then(|chunk| {
            if chunk.is_reduced() {
                Some(chunk)
            } else {
                None
            }
        });

        let mut chunk_id = None;
        let mut accumulator_ids = free_merge_accumulator_ids_vecs.take();
        let mut accumulators_total_data_bytes = 0;

        for chunk in chunks_iter {
            if chunk.is_tombstone() {
                continue;
            }

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
                    let first_chunk = prev_reduced_chunk
                        .as_ref()
                        .expect("already checked")
                        .as_reduced()
                        .expect("already checked");

                    accumulator_ids.push(
                        first_chunk.accumulator_id,
                    );
                    accumulators_total_data_bytes += first_chunk.accumulator_data_bytes;

                    *chunk_id_counter += 1;
                    let new_chunk_id = *chunk_id_counter;

                    chunk_id = Some(new_chunk_id);

                    prev_reduced_chunk = prev_reduced_chunk.map(|prev_reduced_chunk| {
                        *prev_reduced_chunk = TargetKeyChunk::Merging(TargetKeyMergingChunk {
                            chunk_id: new_chunk_id,
                        });

                        prev_reduced_chunk
                    });
                }

                accumulator_ids.push(reduced.accumulator_id);
                accumulators_total_data_bytes += reduced.accumulator_data_bytes;

                *chunk = TargetKeyChunk::Tombstone;
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
            free_merge_accumulator_ids_vecs.push(accumulator_ids);
            return;
        };

        fn is_back_tombstone(chunks: &mut VecDeque<TargetKeyChunk>) -> bool {
            if let Some(chunk) = chunks.back() {
                chunk.is_tombstone()
            } else {
                false
            }
        }

        // Remove tombstones
        while is_back_tombstone(&mut target.chunks) {
            target.chunks.pop_back();
        }

        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::AggregateMerge(
                AggregateMergeEvalAction {
                    target_info: target_info_id,
                    accumulator_ids,
                },
            )),
            HandlerContext::Merging(MergingContext {
                target_key_rc,
                chunk_id,
                accumulators_total_data_bytes,
            }),
            input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                let ctx = ctx.into_merging().expect("should be MergingContext");
                let FunctionEvalInput { body } = input.into_eval_aggregate_merge()?;
                this.on_merge_received(ctx, body)
            }),
        ));
    }
}
