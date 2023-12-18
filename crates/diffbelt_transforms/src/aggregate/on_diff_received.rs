use crate::aggregate::context::{
    HandlerContext, HandlerContextKind, HandlerContextMapError, MapContext,
};
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateMapEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::error::TransformError;
use crate::base::input::function_eval::FunctionEvalInput;
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
    pub fn on_diff_received(
        &mut self,
        diff: DiffCollectionResponseJsonData,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        let DiffCollectionResponseJsonData {
            from_generation_id: _,
            to_generation_id: _,
            items,
            cursor_id,
        } = diff;

        let input = self
            .free_map_eval_action_buffers
            .provide_as_option(|buffer| {
                let mut serializer =
                    Serializer::from_vec(buffer.take().expect("buffers pool should provide Some"));

                let result = try_or_return_with_buffer_back!(
                    (|| {
                        let mut records = Vec::with_capacity(items.len());

                        for item in items {
                            let KeyValueDiffJsonData {
                                key,
                                from_value,
                                intermediate_values: _,
                                to_value,
                            } = item;

                            let source_key = key.into_bytes()?;
                            let source_old_value =
                                from_value.and_then(|x| x).map(|x| x.into_bytes());
                            let source_old_value = lift_result_from_option(source_old_value)?;
                            let source_new_value = to_value.and_then(|x| x).map(|x| x.into_bytes());
                            let source_new_value = lift_result_from_option(source_new_value)?;

                            let source_key = Some(serializer.create_vector(&source_key));
                            let source_old_value =
                                source_old_value.map(|x| serializer.create_vector(&x));
                            let source_new_value =
                                source_new_value.map(|x| serializer.create_vector(&x));

                            let map_source = AggregateMapSource::create(
                                serializer.buffer_builder(),
                                &AggregateMapSourceArgs {
                                    source_key,
                                    source_old_value,
                                    source_new_value,
                                },
                            );

                            records.push(map_source);
                        }

                        let records = serializer.create_vector(&records);

                        let result = AggregateMapMultiInput::create(
                            serializer.buffer_builder(),
                            &AggregateMapMultiInputArgs {
                                items: Some(records),
                            },
                        );

                        Ok::<_, TransformError>(result)
                    })(),
                    buffer,
                    serializer.into_vec()
                );

                let result = serializer.finish(result).into_owned();

                Ok::<_, TransformError>(result)
            })?;

        let output_buffer = self.free_map_eval_input_buffers.take();

        let input_bytes_len = input.as_bytes().len();
        state.current_limits.pending_eval_map_bytes += input_bytes_len;

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::AggregateMap(AggregateMapEvalAction {
                input,
                output_buffer,
            })),
            HandlerContext::Map(MapContext {
                bytes_to_free: input_bytes_len,
            }),
            input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                let FunctionEvalInput { body } = input.into_eval_aggregate_map()?;
                let ctx = ctx
                    .into_map()
                    .map_err_self_to_transform_err(HandlerContextKind::Map)?;
                this.on_map_received(ctx, body)
            }),
        ));

        () = Self::maybe_read_cursor(
            &mut actions,
            &self.max_limits,
            &state.current_limits,
            &self.from_collection_name,
            &mut state.cursor_id,
            cursor_id,
        );

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
