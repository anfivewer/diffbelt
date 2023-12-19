use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use crate::aggregate::state::ProcessingState;
use crate::aggregate::AggregateTransform;
use crate::transform::ActionInputHandlerActionsVec;

impl AggregateTransform {
    pub fn try_apply(
        actions: &mut ActionInputHandlerActionsVec<Self, HandlerContext>,
        max_limits: &Limits,
        current_limits: &Limits,
    ) {
        if !Self::can_eval_apply(max_limits, current_limits) {
            return;
        }

        todo!()
    }
}
