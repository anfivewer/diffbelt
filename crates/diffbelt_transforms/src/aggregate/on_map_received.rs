use std::ops::DerefMut;
use std::rc::Rc;

use diffbelt_protos::protos::transform::aggregate::{AggregateReduceItem, AggregateReduceItemArgs, AggregateTargetInfo};
use diffbelt_protos::Serializer;

use crate::aggregate::context::{HandlerContext, MapContext};
use crate::aggregate::state::TargetKeyData;
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{
    AggregateInitialAccumulatorEvalAction, FunctionEvalAction,
};
use crate::base::action::ActionType;
use crate::base::error::TransformError;
use crate::base::input::function_eval::AggregateMapEvalInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerActionsVec, ActionInputHandlerResult, HandlerResult};

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

        self.free_map_eval_action_buffers.push(action_input_buffer);

        let map_output = input.data();
        let map_items = map_output.items().unwrap_or_default();

        let mut prev_key_target: Option<(&[u8], &mut TargetKeyData)> = None;

        let mut updated_keys = state.updated_target_keys_temp_set.temp();
        let updated_keys = updated_keys.as_mut();

        for item in map_items {
            let target_key = item
                .target_key()
                .ok_or_else(|| TransformError::Unspecified("target_key is None".to_string()))?
                .bytes();
            let mapped_value = item.mapped_value();

            let target: &mut TargetKeyData = 'get_target: {
                if let Some((prev_target_key, target)) = &mut prev_key_target {
                    if *prev_target_key == target_key {
                        break 'get_target target.deref_mut();
                    }
                }

                let target = state.target_keys.get_mut(target_key);
                let target = if let Some(target) = target {
                    target
                } else {
                    state.target_keys.push(
                        Rc::from(target_key),
                        TargetKeyData {
                            reduce_input: Serializer::new(),
                            reduce_input_items: Vec::new(),
                            accumulator_and_target_info: None,
                            is_accumulator_and_target_info_pending: false,
                        },
                    );
                    state
                        .target_keys
                        .get_mut(target_key)
                        .expect("should be inserted")
                };

                prev_key_target = Some((target_key, target));
                let Some((_, target)) = &mut prev_key_target else {
                    panic!()
                };

                target.deref_mut()
            };

            if !target.is_accumulator_and_target_info_pending {
                updated_keys.insert(target_key);
            }

            let prev_buffer_len = target.reduce_input.buffer_len();

            let mapped_value = mapped_value.map(|x| target.reduce_input.create_vector(x.bytes()));

            let item = AggregateReduceItem::create(
                target.reduce_input.buffer_builder(),
                &AggregateReduceItemArgs { mapped_value },
            );

            target.reduce_input_items.push(item);

            let new_buffer_len = target.reduce_input.buffer_len();
            let new_data_size = new_buffer_len - prev_buffer_len;

            state.current_limits.target_data_bytes += new_data_size;
        }

        let mut actions = ActionInputHandlerActionsVec::with_capacity(updated_keys.len());

        for target_key in updated_keys.iter() {
            let target_key = *target_key;

            let target = state
                .target_keys
                .get_mut(target_key)
                .expect("should be present");

            assert!(
                !target.is_accumulator_and_target_info_pending,
                "Accumulator should not be already pending"
            );

            let Some((accumulator_id, target_info_id)) = &target.accumulator_and_target_info else {
                let target_info = Serializer::<AggregateTargetInfo>::new();

                // TODO: We need to make request for target record first

                actions.push((
                    ActionType::FunctionEval(FunctionEvalAction::AggregateInitialAccumulator(
                        AggregateInitialAccumulatorEvalAction { target_info: () },
                    )),
                    HandlerContext::None,
                    input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                        todo!()
                    }),
                ));
                continue;
            };
        }

        todo!()
        // Ok(ActionInputHandlerResult::AddActions(actions))
    }
}
