use std::mem;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use diffbelt_cli_config::wasm::memory::vector::WasmVecHolder;
use diffbelt_cli_config::wasm::{MapFilterFunction, WasmError, WasmModuleInstance};
use diffbelt_protos::align_util::OwnedAlignedBytes;
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiOutput;
use diffbelt_protos::{deserialize, OwnedSerialized};
use diffbelt_transforms::base::action::function_eval::{FunctionEvalAction, MapFilterEvalAction};
use diffbelt_transforms::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use diffbelt_transforms::Transform;
use diffbelt_util::errors::NoStdErrorWrap;

use crate::commands::errors::{CommandError, TransformEvalError};
use crate::commands::transform::run::function_eval_handler::FunctionEvalHandler;
use crate::commands::transform::run::InputEmitter;

pub struct MapFilterEvalHandler {
    verbose: bool,
    inner: Inner,
    instance: Box<WasmModuleInstance>,
}

struct Inner {
    vec_holder: WasmVecHolder<'static>,
    map_filter: MapFilterFunction<'static>,
}

impl MapFilterEvalHandler {
    pub async fn new(
        instance: WasmModuleInstance,
        map_filter_function_name: &str,
        verbose: bool,
    ) -> Result<Self, CommandError> {
        let instance = Box::new(instance);
        let instance_static = unsafe {
            mem::transmute::<&WasmModuleInstance, &'static WasmModuleInstance>(instance.deref())
        };

        let map_filter = instance_static
            .map_filter_function(map_filter_function_name)
            .await?;
        let vec_holder = instance_static.alloc_vec_holder().await?;

        Ok(Self {
            verbose,
            inner: Inner {
                vec_holder,
                map_filter,
            },
            instance,
        })
    }
}

impl FunctionEvalHandler for MapFilterEvalHandler {
    async fn handle_action(
        &self,
        action: FunctionEvalAction,
        input_emitter: InputEmitter,
        _transform: Arc<Mutex<impl Transform>>,
    ) {
        let action = match action.into_map_filter() {
            Ok(action) => action,
            Err(_) => {
                input_emitter
                    .emit_input(Err(TransformEvalError::Unspecified(
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
                .inner
                .map_filter
                .call(input.as_bytes(), &self.inner.vec_holder)
                .await?;

            let aligned_bytes = output.observe_bytes(|bytes| {
                let aligned_bytes = OwnedAlignedBytes::copy_slice(outputs_buffer, bytes)
                    .map_err(NoStdErrorWrap)?;

                // just validate
                let output = deserialize::<MapFilterMultiOutput>(aligned_bytes.as_ref())
                    .map_err(NoStdErrorWrap)?;
                let Some(_records) = output.target_update_records() else {
                    return Err(TransformEvalError::Unspecified(
                        "map_filter function did not returned event empty target_update_records"
                            .to_string(),
                    ));
                };

                Ok::<_, TransformEvalError>(aligned_bytes)
            })?;

            let output =
                OwnedSerialized::<MapFilterMultiOutput<'static>>::from_aligned_bytes(aligned_bytes)
                    .map_err(NoStdErrorWrap)?;

            Ok(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    input: output,
                    action_input_buffer: input.into_buffer_vec(),
                }),
            })
        })()
        .await;

        () = input_emitter.emit_input(result).await;
    }
}
