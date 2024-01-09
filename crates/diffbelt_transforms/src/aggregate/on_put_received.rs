use diffbelt_types::collection::put_many::PutManyResponseJsonData;

use crate::aggregate::AggregateTransform;
use crate::aggregate::context::{ApplyingPutContext, HandlerContext};
use crate::aggregate::state::TargetKeyApplying;
use crate::transform::HandlerResult;

impl AggregateTransform {
    pub fn on_put_received(
        &mut self,
        ctx: ApplyingPutContext,
        _put: PutManyResponseJsonData,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        state.current_limits.pending_puts_count -= 1;

        let ApplyingPutContext { target_keys } = ctx;

        for target_key in target_keys.iter() {
            let target = state
                .target_keys
                .get_mut(target_key)
                .expect("target should exist while applying")
                .as_applying_mut()
                .expect("should be applying while applying");

            let TargetKeyApplying {
                mapped_values,
                is_got_value: _,
                is_putting: _,
                target_kv_size,
            } = target;

            state.current_limits.pending_applying_bytes -= *target_kv_size;

            if !mapped_values.is_empty() {
                // TODO: move value in `on_apply_received` to TargetKeyApplying from `apply_puts`
                todo!("create accumulator, start reduce");
            }

            _ = state.target_keys.pop(target_key);
        }

        self.free_target_keys_buffers.push(target_keys);

        self.on_finish()
    }
}
