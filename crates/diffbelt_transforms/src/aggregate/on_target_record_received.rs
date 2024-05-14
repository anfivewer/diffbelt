use std::rc::Rc;

use diffbelt_protos::protos::transform::aggregate::{AggregateTargetInfo, AggregateTargetInfoArgs};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::get_record::GetResponseJsonData;
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::buffers_pool::BuffersPool;

use crate::aggregate::context::{HandlerContext, TargetRecordContext};
use crate::aggregate::state::Target;
use crate::aggregate::AggregateTransform;
use crate::base::action::function_eval::{AggregateTargetInfoEvalAction, FunctionEvalAction};
use crate::base::action::ActionType;
use crate::base::input::function_eval::FunctionEvalInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerActionsVec, ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn on_target_record_received(
        &mut self,
        ctx: TargetRecordContext,
        body: GetResponseJsonData,
    ) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        let TargetRecordContext {
            target_key: target_key_rc,
        } = ctx;
        let GetResponseJsonData {
            generation_id: _,
            item,
        } = body;

        let target_old_value = item.map(|item| item.value.into_bytes());
        let target_old_value = lift_result_from_option(target_old_value)?;

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        let mut target = state
            .target_keys
            .get_mut(&target_key_rc)
            .expect("target cannot be removed while there is pending get target record");

        handle_received_target_record(
            &mut actions,
            &mut target,
            target_key_rc,
            target_old_value,
            &mut self.free_target_info_action_buffers,
        );

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}

pub fn handle_received_target_record(
    actions: &mut ActionInputHandlerActionsVec<AggregateTransform, HandlerContext>,
    target: &mut Target,
    target_key_rc: Rc<[u8]>,
    target_old_value: Option<Box<[u8]>>,
    free_target_info_action_buffers: &mut BuffersPool<Vec<u8>>,
) {
    let target = target
        .as_processing_mut()
        .expect("target cannot be applied while there is pending target info");

    assert!(
        target.is_target_info_pending,
        "there should be no multiple pending get records for same target key"
    );
    assert!(
        target.target_info_id.is_none(),
        "if target info pending, there should be no target info id"
    );

    let buffer = free_target_info_action_buffers.take();
    let mut serializer = Serializer::from_vec(buffer);

    let target_key = serializer.create_vector(&target_key_rc);
    let target_old_value = target_old_value.map(|x| serializer.create_vector(&x));

    let target_info = AggregateTargetInfo::create(
        serializer.buffer_builder(),
        &AggregateTargetInfoArgs {
            target_key: Some(target_key),
            target_old_value,
        },
    );
    let target_info = serializer.finish(target_info).into_owned();

    actions.push((
        ActionType::FunctionEval(FunctionEvalAction::AggregateTargetInfo(
            AggregateTargetInfoEvalAction { target_info },
        )),
        HandlerContext::TargetRecord(TargetRecordContext {
            target_key: target_key_rc,
        }),
        input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
            let ctx = ctx.into_target_record().expect("should be TargetRecord");
            let FunctionEvalInput { body } = input.into_eval_aggregate_target_info()?;
            this.on_target_info_received(ctx, body)
        }),
    ));
}
