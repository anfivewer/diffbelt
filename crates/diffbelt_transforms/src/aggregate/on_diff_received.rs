use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateMapEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::error::TransformError;
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
    ) -> HandlerResult<Self> {
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

        state.pending_eval_bytes += input.as_bytes().len();

        let read_cursor = left_if_some(cursor_id).left_and_then(|cursor| {
            if state.pending_eval_bytes < self.max_pending_bytes {
                Either::Left(cursor)
            } else {
                Either::Right(Some(cursor))
            }
        });

        let need_read_cursor = read_cursor.is_left();

        let mut actions =
            ActionInputHandlerActionsVec::with_capacity(1 + (if need_read_cursor { 1 } else { 0 }));

        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::AggregateMap(AggregateMapEvalAction {
                input,
                output_buffer,
            })),
            input_handler!(this, AggregateTransform, input, { todo!() }),
        ));

        match read_cursor {
            Either::Left(cursor) => {
                actions.push(Self::read_cursor(&self.from_collection_name, &cursor));
            }
            Either::Right(cursor) => {
                state.cursor_id = cursor;
            }
        }

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
