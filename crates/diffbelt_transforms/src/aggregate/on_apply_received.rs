use diffbelt_types::collection::put_many::PutManyRequestJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_types::common::key_value_update::KeyValueUpdateJsonData;
use diffbelt_util_no_std::cast::usize_to_u64;
use std::borrow::Cow;

use crate::aggregate::context::{ApplyingContext, HandlerContext};
use crate::aggregate::state::TargetKeyApplying;
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::ActionType;
use crate::base::input::function_eval::AggregateApplyEvalInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerResult, HandlerResult};

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

        let target_value_size = target_value
            .as_ref()
            .map(|value| usize_to_u64(value.len()))
            .unwrap_or(0);

        state.current_limits.applying_bytes -= applying_bytes;
        state.current_limits.pending_applying_bytes += target_value_size;

        let needs_do_put = state.current_limits.pending_applying_bytes
            >= self.max_limits.pending_applying_bytes
            || state.current_limits.pending_applies_count == 0;

        if needs_do_put {
            let mut actions = self.action_input_handlers.take_action_input_actions_vec();

            let items = vec![KeyValueUpdateJsonData {
                key: EncodedKeyJsonData::from_bytes_slice(&target_key),
                if_not_present: None,
                value: target_value.map(|value| EncodedValueJsonData::from_bytes_slice(&value)),
            }];

            // Do puts_many
            actions.push((
                ActionType::DiffbeltCall(DiffbeltCallAction {
                    method: Method::Post,
                    path: Cow::Owned(format!(
                        "/collections/{}/putMany",
                        urlencoding::encode(&self.to_collection_name),
                    )),
                    query: Vec::with_capacity(0),
                    body: DiffbeltRequestBody::PutMany(PutManyRequestJsonData {
                        items,
                        generation_id: Some(state.to_generation_id.clone()),
                        phantom_id: None,
                    }),
                }),
                HandlerContext::None,
                input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                    todo!()
                }),
            ));

            return Ok(ActionInputHandlerResult::AddActions(actions));
        } else {
            state.apply_puts.insert(target_key, target_value);
        }

        Ok(ActionInputHandlerResult::Consumed)
    }
}
