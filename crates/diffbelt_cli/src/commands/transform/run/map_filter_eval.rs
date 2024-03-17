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

pub struct MapFilterEvalHandler {
    pub verbose: bool,
    pub instance: *const WasmModuleInstance,
    pub vec_holder: WasmVecHolder<'static>,
    pub map_filter: MapFilterFunction<'static>,
}

impl FunctionEvalHandler for MapFilterEvalHandler {
    async fn handle_action<
        'a,
        Fut: Future<Output = ()>,
        F: Fn(Result<FunctionEvalInput<FunctionEvalInputBody>, TransformEvalError>) -> Fut,
    >(
        &self,
        action: FunctionEvalAction,
        emit_input: &F,
    ) {
        let action = match action.into_map_filter() {
            Ok(action) => action,
            Err(_) => {
                emit_input(Err(TransformEvalError::Unspecified(
                    "action is not MapFilterEvalAction".to_string(),
                )))
                .await;
                return;
            }
        };

        let MapFilterEvalAction {
            input,
            output_buffer: mut outputs_buffer,
        } = action;

        let result = (|| async move {
            let output = self
                .map_filter
                .call(input.as_bytes(), &self.vec_holder)
                .await?;

            () = output.observe_bytes(|bytes| {
                // just validate
                let output = deserialize::<MapFilterMultiOutput>(bytes).map_err(NoStdErrorWrap)?;
                let Some(_records) = output.target_update_records() else {
                    return Err(TransformEvalError::Unspecified(
                        "map_filter function did not returned event empty target_update_records"
                            .to_string(),
                    ));
                };

                outputs_buffer.clear();
                outputs_buffer.extend_from_slice(bytes);

                Ok::<_, TransformEvalError>(())
            })?;

            let output = OwnedSerialized::<MapFilterMultiOutput<'static>>::from_vec(outputs_buffer)
                .map_err(NoStdErrorWrap)?;

            Ok(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    input: output,
                    action_input_buffer: input.into_vec(),
                }),
            })
        })()
        .await;

        () = emit_input(result).await;
    }
}
