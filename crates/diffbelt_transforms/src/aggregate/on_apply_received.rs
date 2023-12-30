use crate::aggregate::context::{
    ApplyingContext, HandlerContext, HandlerContextKind, HandlerContextMapError, MapContext,
};
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateMapEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::error::TransformError;
use crate::base::input::function_eval::{AggregateApplyEvalInput, FunctionEvalInput};
use crate::input_handler;
use crate::transform::{ActionInputHandlerActionsVec, ActionInputHandlerResult, HandlerResult};
use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiInput, AggregateMapMultiInputArgs, AggregateMapSource, AggregateMapSourceArgs,
};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::diff::{DiffCollectionResponseJsonData, KeyValueDiffJsonData};
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::cast::usize_to_u64;
use diffbelt_util_no_std::either::left_if_some;
use diffbelt_util_no_std::from_either::Either;
use diffbelt_util_no_std::try_or_return_with_buffer_back;

impl AggregateTransform {
    pub fn on_apply_received(
        &mut self,
        ctx: ApplyingContext,
        apply: AggregateApplyEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        state.current_limits.pending_applies_count -= 1;

        let ApplyingContext {
            target_key,
            applying_bytes,
        } = ctx;
        let AggregateApplyEvalInput { input } = apply;

        let target = state
            .target_keys
            .get_mut(&target_key)
            .expect("target should exist while applying")
            .as_applying_mut()
            .expect("should be applying while applying");

        target.is_got_value = true;

        let apply = input.data();
        let target_value = apply
            .target_value()
            .map(|value| Box::<[u8]>::from(value.bytes()));

        if !target.mapped_values.is_empty() {
            // Do not put save result, resume reducing
            todo!()
        }

        state.current_limits.applying_bytes -= applying_bytes;
        state.current_limits.pending_applying_bytes += target_value
            .as_ref()
            .map(|value| usize_to_u64(value.len()))
            .unwrap_or(0);

        state.apply_puts.insert(target_key, target_value);

        let needs_do_put = state.current_limits.pending_applying_bytes
            >= self.max_limits.pending_applying_bytes
            || state.current_limits.pending_applies_count == 0;

        if needs_do_put {
            // Do puts_many
            todo!()
        }

        Ok(ActionInputHandlerResult::Consumed)
    }
}
