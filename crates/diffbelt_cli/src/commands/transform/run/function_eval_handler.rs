use std::future::Future;

use enum_dispatch::enum_dispatch;

use diffbelt_transforms::base::action::function_eval::FunctionEvalAction;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};
use diffbelt_transforms::Transform;

use crate::commands::errors::TransformEvalError;
use crate::commands::transform::run::aggregate_eval::AggregateEvalHandler;
use crate::commands::transform::run::map_filter_eval::MapFilterEvalHandler;

#[enum_dispatch]
pub trait FunctionEvalHandler {
    async fn handle_action<
        'a,
        Fut: Future<Output = ()>,
        F: Fn(Result<FunctionEvalInput<FunctionEvalInputBody>, TransformEvalError>) -> Fut,
    >(
        &self,
        action: FunctionEvalAction,
        emit_input: &F,
        transform: &'a mut impl Transform,
    );
}

#[enum_dispatch(FunctionEvalHandler)]
pub enum FunctionEvalHandlerImpl {
    MapFilter(MapFilterEvalHandler),
    Aggregate(AggregateEvalHandler),
}
