use crate::commands::errors::TransformEvalError;
use crate::commands::transform::runner::create_transform::{
    create_transform, TransformDirection, TransformEvaluator,
};
use crate::commands::transform::runner::function_eval_handler::{
    FunctionEvalHandler, FunctionEvalHandlerImpl,
};
use crate::commands::transform::runner::transform_config::{TransformConfig, TransformInfo};
use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::wasm::cli_api::WasmCliApi;
use diffbelt_cli_config::wasm::engine::{WasmEngine, WasmEngineOptions};
use diffbelt_cli_config::{CliConfig, Collection};
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_transforms::base::action::{Action, ActionType};
use diffbelt_transforms::base::input::diffbelt_call::DiffbeltCallInput;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_transforms::{Transform, TransformImpl, TransformRunResult};
use diffbelt_util::Wrap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::{spawn_local, yield_now, LocalSet};

mod aggregate_eval;
mod create_transform;
mod function_eval_handler;
mod map_filter_eval;
mod parse;
pub mod transform_config;

pub struct TransformRunner {
    config: TransformConfig,
    wasm_engine: WasmEngine,
    client: Arc<DiffbeltClient>,
    cli_api: WasmCliApi,
}

impl TransformRunner {
    pub fn new(
        config: TransformConfig,
        wasm_engine: WasmEngine,
        client: Arc<DiffbeltClient>,
        cli_api: WasmCliApi,
    ) -> Self {
        Self {
            config,
            wasm_engine,
            client,
            cli_api,
        }
    }

    pub async fn run_transform(&self, name: &str) -> Result<(), RunTransformError> {
        let transform_config = self.config.transforms.get(name).ok_or_else(|| {
            RunTransformError::Message(format!("No transform with name \"{name}\""))
        })?;

        let TransformInfo {
            source_collection_name,
            intermediate_collection_name,
            target_collection_name,
            reader_name,
            ..
        } = transform_config;

        if let Some(_) = intermediate_collection_name {
            return Err(RunTransformError::Message(
                "Transforms with intermediate collection are not supported yet".to_string(),
            ));
        }

        let reader_name = reader_name.as_ref().map(|x| x.deref());

        let Some(reader_name) = reader_name else {
            return Err(RunTransformError::Message(
                "No reader_name present".to_string(),
            ));
        };

        if let Some(to_collection_from_collections) =
            self.config.collections.get(target_collection_name.as_str())
        {
            if !to_collection_from_collections.is_manual {
                return Err(RunTransformError::Message(format!(
                    "Collection \"{target_collection_name}\" is not manual"
                )));
            }
        }

        let transform_direction = TransformDirection {
            from_collection_name: source_collection_name.as_str(),
            to_collection_name: target_collection_name.as_str(),
            reader_name,
        };

        let TransformEvaluator {
            transform,
            eval_handler,
            requests_impl,
        } = create_transform(
            &self.wasm_engine,
            self.client.clone(),
            self.cli_api.clone(),
            transform_config,
            transform_direction,
        )
        .await?;

        // TODO: thread pool, parallelize function evals and diffbelt calls
        //       (they parse/serialize jsons currently)

        let mut inputs = Vec::new();
        let (sender, mut receiver) = mpsc::channel::<Result<Input, RunTransformError>>(8);

        let transform = Wrap::wrap(transform);
        let eval_handler = Wrap::wrap(eval_handler);

        let local = LocalSet::new();

        let () = local
            .run_until(async {
                loop {
                    let need_continue = processing_iteration(
                        &transform,
                        &eval_handler,
                        &mut inputs,
                        &sender,
                        &mut receiver,
                        &self.client,
                    )
                    .await?;

                    if !need_continue {
                        break;
                    }
                }

                Ok::<(), RunTransformError>(())
            })
            .await?;

        drop(requests_impl);

        Ok(())
    }
}

pub struct InputEmitter {
    action_id: (u64, u64),
    sender: mpsc::Sender<Result<Input, RunTransformError>>,
}

impl InputEmitter {
    pub async fn emit_input(
        &self,
        input_or_error: Result<FunctionEvalInput<FunctionEvalInputBody>, TransformEvalError>,
    ) {
        let result = input_or_error.map_or_else(
            |err| Err(err.into()),
            |input| {
                Ok(Input {
                    id: self.action_id,
                    input: InputType::FunctionEval(input),
                })
            },
        );

        let mut msg = result;

        let () = loop {
            match self.sender.try_send(msg) {
                Ok(()) => {
                    break ();
                }
                Err(err) => match err {
                    TrySendError::Full(data) => {
                        msg = data;
                        // let current thread process messages
                        yield_now().await;
                    }
                    TrySendError::Closed(_) => {
                        break ();
                    }
                },
            }
        };
    }
}

async fn processing_iteration(
    transform: &Arc<Mutex<TransformImpl>>,
    eval_handler: &Arc<FunctionEvalHandlerImpl>,
    inputs: &mut Vec<Input>,
    sender: &mpsc::Sender<Result<Input, RunTransformError>>,
    receiver: &mut mpsc::Receiver<Result<Input, RunTransformError>>,
    client: &Arc<DiffbeltClient>,
) -> Result<bool, RunTransformError> {
    let run_result = {
        let mut transform = transform.lock().expect("lock");
        let result = transform.run(inputs)?;
        inputs.clear();

        result
    };

    match run_result {
        TransformRunResult::Actions(mut actions) => {
            for action in actions.drain(..) {
                let Action {
                    id: action_id,
                    action,
                } = action;

                let input_emitter = InputEmitter {
                    action_id,
                    sender: sender.clone(),
                };

                match action {
                    ActionType::DiffbeltCall(call) => {
                        let sender = sender.clone();
                        let client = client.clone();
                        tokio::spawn(async move {
                            let message = match client.transform_call(call).await {
                                Ok(body) => Ok(Input {
                                    id: action_id,
                                    input: InputType::DiffbeltCall(DiffbeltCallInput { body }),
                                }),
                                Err(err) => Err(err.into()),
                            };

                            let () = sender.send(message).await.unwrap_or(());
                        });
                    }
                    ActionType::FunctionEval(eval) => {
                        let eval_handler = eval_handler.clone();
                        let transform = transform.clone();
                        spawn_local(async move {
                            let () = eval_handler
                                .handle_action(eval, input_emitter, transform)
                                .await;
                        });
                    }
                }
            }

            {
                let mut transform = transform.lock().expect("lock");
                transform.return_actions_vec(actions)
            }
        }
        TransformRunResult::Finish => {
            return Ok(false);
        }
    }

    let () = receive_inputs(inputs, receiver).await?;

    Ok(true)
}

async fn receive_inputs(
    inputs: &mut Vec<Input>,
    receiver: &mut mpsc::Receiver<Result<Input, RunTransformError>>,
) -> Result<(), RunTransformError> {
    if !inputs.is_empty() {
        return Ok(());
    }

    let input = receiver.recv().await;

    let Some(input) = input else {
        return Err(RunTransformError::Message(
            "Inputs channel was closed".to_string(),
        ));
    };

    let input = input?;

    inputs.push(input);

    while let Ok(input) = receiver.try_recv() {
        let input = input?;
        inputs.push(input);
    }

    Ok(())
}
