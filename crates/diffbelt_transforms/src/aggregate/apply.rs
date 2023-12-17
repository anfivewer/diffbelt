use crate::aggregate::AggregateTransform;
use crate::aggregate::context::HandlerContext;
use crate::transform::ActionInputHandlerActionsVec;

impl AggregateTransform {
    pub fn try_apply(actions: &mut ActionInputHandlerActionsVec<Self, HandlerContext>,) {
        // TODO: if limit on target data is reached and there is single chunk, or we are finished, do apply
    }
}
