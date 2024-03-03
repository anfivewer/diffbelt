use std::future::Future;

use diffbelt_cli_config::wasm::memory::vector::WasmVecHolder;
use diffbelt_cli_config::wasm::{MapFilterFunction, WasmModuleInstance};
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiOutput;
use diffbelt_protos::{deserialize, OwnedSerialized};
use diffbelt_transforms::base::action::function_eval::{FunctionEvalAction, MapFilterEvalAction};
use diffbelt_transforms::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use diffbelt_util::errors::NoStdErrorWrap;

use crate::commands::errors::TransformEvalError;
use crate::commands::transform::run::function_eval_handler::FunctionEvalHandler;

pub struct AggregateEvalHandler {
    pub verbose: bool,
    pub instance: *const WasmModuleInstance,
}

impl AggregateEvalHandler {
    //
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
        /*
    AggregateMap(AggregateMapEvalAction),
    AggregateTargetInfo(AggregateTargetInfoEvalAction),
    AggregateInitialAccumulator(AggregateInitialAccumulatorEvalAction),
    AggregateReduce(AggregateReduceEvalAction),
    AggregateMerge(AggregateMergeEvalAction),
    AggregateApply(AggregateApplyEvalAction),
         */

        todo!()
    }
}
