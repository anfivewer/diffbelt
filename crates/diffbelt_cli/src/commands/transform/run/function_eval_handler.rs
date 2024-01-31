use crate::commands::errors::TransformEvalError;
use crate::commands::transform::run::map_filter_eval::MapFilterEvalHandler;
use diffbelt_transforms::base::action::function_eval::FunctionEvalAction;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};
use enum_dispatch::enum_dispatch;
use std::future::Future;

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
    );
}

#[enum_dispatch(FunctionEvalHandler)]
pub enum FunctionEvalHandlerImpl {
    MapFilter(MapFilterEvalHandler),
}
