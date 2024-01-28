use crate::commands::errors::TransformEvalError;
use diffbelt_cli_config::wasm::memory::WasmVecHolder;
use diffbelt_transforms::base::action::function_eval::FunctionEvalAction;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};
use std::future::Future;

pub trait FunctionEvalHandler: Sized {
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
