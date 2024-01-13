use std::mem;
use std::rc::Rc;

use lru::LruCache;

use diffbelt_util_no_std::buffers_pool::BuffersPool;
use diffbelt_util_no_std::temporary_collection::vec::TemporaryVec;

use crate::aggregate::context::{ApplyingContext, HandlerContext};
use crate::aggregate::limits::Limits;
use crate::aggregate::state::{Target, TargetKeyApplying, TargetKeyReducedChunk, TargetKvTemp};
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateApplyEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::input::function_eval::FunctionEvalInput;
use crate::input_handler;
use crate::transform::ActionInputHandlerActionsVec;

impl AggregateTransform {
    pub fn try_apply(
        actions: &mut ActionInputHandlerActionsVec<Self, HandlerContext>,
        max_limits: &Limits,
        current_limits: &mut Limits,
        target_keys: &mut LruCache<Rc<[u8]>, Target>,
        temp_targets: &mut TemporaryVec<TargetKvTemp>,
        free_apply_eval_buffers: &mut BuffersPool<Vec<u8>>,
    ) {
        if !Self::can_eval_apply(max_limits, current_limits) {
            return;
        }

        let is_needed = Self::need_eval_apply(max_limits, current_limits);

        let mut targets_to_apply_temp = temp_targets.temp();
        let target_to_apply = targets_to_apply_temp.as_mut();

        if !is_needed {
            // Finish of diffs, apply all what we can
            for (key, target) in target_keys.iter_mut() {
                let Some(processing) = target.as_processing() else {
                    continue;
                };

                if !processing.target_info_id.is_some() {
                    continue;
                }

                if processing.chunks.len() != 1 {
                    continue;
                }

                let chunk = processing.chunks.get(0).expect("length is checked");

                if !chunk.is_reduced() {
                    continue;
                }

                target_to_apply.push((key.clone(), target));
            }
        } else {
            // TODO: Apply only by LRU with threshold
        }

        for (key, target) in target_to_apply {
            let mut applying = Target::Applying(TargetKeyApplying {
                mapped_values: Vec::with_capacity(0),
                is_got_value: false,
                is_putting: false,
                target_value: None,
                target_kv_size: 0,
            });

            mem::swap(*target, &mut applying);
            let target = &mut applying;

            let target = target.as_processing_mut().expect("filtered before");

            let target_info_id = target.target_info_id.expect("filtered before");

            let TargetKeyReducedChunk {
                accumulator_id,
                accumulator_data_bytes,
            } = target
                .chunks
                .get(0)
                .expect("filtered before")
                .as_reduced()
                .expect("filtered before");

            let data_bytes_count = target.target_info_data_bytes + accumulator_data_bytes;
            current_limits.target_data_bytes -= data_bytes_count;
            current_limits.applying_bytes += data_bytes_count;
            current_limits.pending_applies_count += 1;

            actions.push((
                ActionType::FunctionEval(FunctionEvalAction::AggregateApply(
                    AggregateApplyEvalAction {
                        target_info: target_info_id,
                        accumulator: *accumulator_id,
                        output_buffer: free_apply_eval_buffers.take(),
                    },
                )),
                HandlerContext::Applying(ApplyingContext {
                    target_key: key.clone(),
                    applying_bytes: data_bytes_count,
                }),
                input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                    let ctx = ctx.into_applying().expect("should be ApplyingContext");
                    let FunctionEvalInput { body } = input.into_eval_aggregate_apply()?;
                    this.on_apply_received(ctx, body)
                }),
            ));
        }
    }
}
