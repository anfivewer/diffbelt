use diffbelt_protos::protos::transform::aggregate::{AggregateReduceItem, AggregateReduceItemArgs};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::put_many::PutManyResponseJsonData;
use diffbelt_util_no_std::cast::usize_to_u64;
use std::collections::VecDeque;
use std::mem;

use crate::aggregate::context::{ApplyingPutContext, HandlerContext};
use crate::aggregate::on_target_record_received::handle_received_target_record;
use crate::aggregate::state::{
    Target, TargetKeyApplying, TargetKeyChunk, TargetKeyCollectingChunk, TargetKeyData,
};
use crate::aggregate::AggregateTransform;
use crate::transform::{ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn on_put_received(
        &mut self,
        ctx: ApplyingPutContext,
        _put: PutManyResponseJsonData,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        state.current_limits.pending_puts_count -= 1;

        let ApplyingPutContext { target_keys } = ctx;

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        for target_key in target_keys.iter() {
            let target = state
                .target_keys
                .get_mut(target_key)
                .expect("target should exist while applying");

            let TargetKeyApplying {
                mapped_values,
                is_got_value,
                is_putting,
                target_value,
                target_kv_size,
            } = target
                .as_applying_mut()
                .expect("should be applying while applying");

            let mut empty_mapped_values = Vec::with_capacity(0);
            mem::swap(mapped_values, &mut empty_mapped_values);
            let mapped_values = empty_mapped_values;

            assert!(*is_got_value);
            assert!(*is_putting);

            state.current_limits.pending_applying_bytes -= *target_kv_size;

            if mapped_values.is_empty() {
                _ = state.target_keys.pop(target_key);
                continue;
            }

            let target_value = target_value.take();

            let mut chunks = VecDeque::with_capacity(1);

            let buffer = self.free_reduce_eval_action_buffers.take();
            let mut reduce_input = Serializer::from_vec(buffer);
            let mut reduce_input_items = self.free_serializer_reduce_input_items_buffers.take();

            for mapped_value in mapped_values {
                let mapped_value = mapped_value.map(|x| reduce_input.create_vector(&x));

                let item = AggregateReduceItem::create(
                    reduce_input.buffer_builder(),
                    &AggregateReduceItemArgs { mapped_value },
                );

                reduce_input_items.push(item);
            }

            state.current_limits.pending_reduces_count += 1;
            state.current_limits.target_data_bytes += usize_to_u64(reduce_input.buffer_len());

            chunks.push_back(TargetKeyChunk::Collecting(TargetKeyCollectingChunk {
                accumulator_id: None,
                accumulator_data_bytes: 0,
                is_accumulator_pending: false,
                is_reducing: false,
                reduce_input,
                reduce_input_items,
            }));

            let mut new_target = Target::Processing(TargetKeyData {
                target_info_id: None,
                target_info_data_bytes: 0,
                chunks,
                is_target_info_pending: true,
            });
            mem::swap(target, &mut new_target);

            handle_received_target_record(
                &mut actions,
                target,
                target_key.clone(),
                target_value,
                &mut self.free_target_info_action_buffers,
            );
        }

        self.free_target_keys_buffers.push(target_keys);

        if !actions.is_empty() {
            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        self.action_input_handlers
            .return_action_input_actions_vec(actions);

        self.on_finish()
    }
}
