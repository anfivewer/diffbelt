use diffbelt_cli_config::transforms::aggregate::Aggregate;
use std::future::Future;

use diffbelt_cli_config::wasm::WasmModuleInstance;
use diffbelt_transforms::base::action::function_eval::FunctionEvalAction;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};

use crate::commands::errors::{CommandError, TransformEvalError};
use crate::commands::transform::run::function_eval_handler::FunctionEvalHandler;

pub struct AggregateEvalHandler {
    verbose: bool,
    instance: *const WasmModuleInstance,
}

impl AggregateEvalHandler {
    pub async fn new(
        instance: WasmModuleInstance,
        aggregate: &Aggregate,
        verbose: bool,
    ) -> Result<Self, CommandError> {
        let instance = Box::new(instance);

        //

        let instance = Box::leak(instance);

        Ok(Self {
            verbose,
            instance: instance as *mut WasmModuleInstance,
        })
    }
}

impl FunctionEvalHandler for AggregateEvalHandler {
    async fn handle_action<
        'a,
        Fut: Future<Output = ()>,
        F: Fn(Result<FunctionEvalInput<FunctionEvalInputBody>, TransformEvalError>) -> Fut,
    >(
        &self,
        action: FunctionEvalAction,
        emit_input: &F,
    ) {
        let result = match action {
            FunctionEvalAction::AggregateMap(action) => {
                todo!("map {action:#?}")
            }
            FunctionEvalAction::AggregateTargetInfo(action) => {
                todo!("target info {action:#?}")
            }
            FunctionEvalAction::AggregateInitialAccumulator(action) => {
                todo!("initial accumulator {action:#?}")
            }
            FunctionEvalAction::AggregateReduce(action) => {
                todo!("reduce {action:#?}")
            }
            FunctionEvalAction::AggregateMerge(action) => {
                todo!("merge {action:#?}")
            }
            FunctionEvalAction::AggregateApply(action) => {
                todo!("apply {action:#?}")
            }
            _ => {
                Err::<FunctionEvalInput<FunctionEvalInputBody>, _>(TransformEvalError::Unspecified(
                    format!("Unsupported aggregate action: {action:?}"),
                ))
            }
        };

        () = emit_input(result).await;
    }
}
