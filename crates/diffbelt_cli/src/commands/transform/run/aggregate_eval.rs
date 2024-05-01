use diffbelt_cli_config::transforms::aggregate::Aggregate;
use diffbelt_cli_config::wasm::aggregate::AggregateFunctions;
use std::future::Future;
use std::mem;
use std::ops::Deref;
use std::str::from_utf8;

use diffbelt_cli_config::wasm::{WasmError, WasmModuleInstance};
use diffbelt_protos::protos::transform::aggregate::AggregateMapMultiOutput;
use diffbelt_protos::{OwnedSerialized, SerializedRawParts};
use diffbelt_transforms::base::action::function_eval::{
    AggregateMapEvalAction, FunctionEvalAction,
};
use diffbelt_transforms::base::input::function_eval::{
    AggregateMapEvalInput, FunctionEvalInput, FunctionEvalInputBody,
};
use diffbelt_transforms::Transform;
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;

use crate::commands::errors::{CommandError, TransformEvalError, WasmAggregateMapCallError};
use crate::commands::transform::run::function_eval_handler::FunctionEvalHandler;

pub struct AggregateEvalHandler {
    verbose: bool,
    inner: Inner,
    instance: Box<WasmModuleInstance>,
}

struct Inner {
    aggregate_functions: AggregateFunctions<'static>,
}

impl AggregateEvalHandler {
    pub async fn new(
        instance: WasmModuleInstance,
        aggregate: &Aggregate,
        verbose: bool,
    ) -> Result<Self, CommandError> {
        let instance = Box::new(instance);
        let instance_static = unsafe {
            mem::transmute::<&WasmModuleInstance, &'static WasmModuleInstance>(instance.deref())
        };

        let aggregate_functions = AggregateFunctions::new(
            instance_static,
            aggregate.map.as_str(),
            aggregate.initial_accumulator.as_str(),
            aggregate.reduce.as_str(),
            aggregate.merge_accumulators.as_ref().map(|x| x.as_str()),
            aggregate.apply.as_str(),
        )
        .await?;

        Ok(Self {
            verbose,
            inner: Inner {
                aggregate_functions,
            },
            instance,
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
        transform: &'a mut impl Transform,
    ) {
        let body = (|| async move {
            match action {
                FunctionEvalAction::AggregateMap(action) => {
                    let AggregateMapEvalAction { input } = action;

                    let output_buffer = transform.take_map_input_buffer();
                    let mut output_holder = Some(output_buffer);

                    let result = self
                        .inner
                        .aggregate_functions
                        .call_map(
                            FlatbufferAnnotated::from(input.as_bytes()),
                            &mut output_holder,
                        )
                        .await;

                    let result = match result {
                        Ok(x) => x,
                        Err(err) => {
                            let SerializedRawParts { buffer, head, len } = input.into_raw_parts();

                            return Err(TransformEvalError::WasmAggregateMapCall(
                                WasmAggregateMapCallError {
                                    input_buffer: buffer,
                                    input_head: head,
                                    input_len: len,
                                    error: err,
                                },
                            ));
                        }
                    };

                    transform.return_map_action_buffer(input.into_buffer_vec());

                    Ok(FunctionEvalInputBody::AggregateMap(AggregateMapEvalInput {
                        input: result,
                    }))
                }
                FunctionEvalAction::AggregateTargetInfo(action) => {
                    todo!("target info")
                }
                FunctionEvalAction::AggregateInitialAccumulator(action) => {
                    todo!("initial accumulator")
                }
                FunctionEvalAction::AggregateReduce(action) => {
                    todo!("reduce")
                }
                FunctionEvalAction::AggregateMerge(action) => {
                    todo!("merge")
                }
                FunctionEvalAction::AggregateApply(action) => {
                    todo!("apply")
                }
                _ => Err(TransformEvalError::Unspecified(format!(
                    "Unsupported aggregate action: {action:?}"
                ))),
            }
        })()
        .await;

        () = emit_input(body.map(|body| FunctionEvalInput { body })).await;
    }
}
