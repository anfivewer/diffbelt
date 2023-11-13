use crate::aggregate::context::{HandlerContext, MapContext};
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateMapEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::error::TransformError;
use crate::base::input::function_eval::AggregateMapEvalInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerActionsVec, ActionInputHandlerResult, HandlerResult};
use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiInput, AggregateMapMultiInputArgs, AggregateMapSource, AggregateMapSourceArgs,
};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::diff::{DiffCollectionResponseJsonData, KeyValueDiffJsonData};
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::either::left_if_some;
use diffbelt_util_no_std::from_either::Either;
use diffbelt_util_no_std::try_or_return_with_buffer_back;

impl AggregateTransform {
    pub fn on_map_received(
        &mut self,
        ctx: MapContext,
        map: AggregateMapEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        let MapContext { bytes_to_free } = ctx;
        state.current_limits.pending_eval_map_bytes -= bytes_to_free;

        let AggregateMapEvalInput {
            input,
            action_input_buffer,
        } = map;

        let map_output = input.data();
        let map_items = map_output.items().unwrap_or_default();

        for item in map_items {
            let old_item = item.old_item();
            let new_item = item.new_item();

            todo!()
        }

        todo!()
    }
}
