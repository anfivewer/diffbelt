use std::ops::Deref;
use std::sync::{Arc, Mutex};

use clap::Args;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::{spawn_local, yield_now, LocalSet};

use diffbelt_cli_config::transforms::Transform as TransformConfig;
use diffbelt_cli_config::Collection;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_transforms::base::action::{Action, ActionType};
use diffbelt_transforms::base::input::diffbelt_call::DiffbeltCallInput;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_transforms::{Transform, TransformImpl, TransformRunResult};
use diffbelt_util::Wrap;

use crate::commands::errors::{CommandError, TransformEvalError};
use crate::commands::transform::run::create_transform::{
    create_transform, TransformDirection, TransformEvaluator,
};
use crate::commands::transform::run::function_eval_handler::{
    FunctionEvalHandler, FunctionEvalHandlerImpl,
};
use crate::state::CliState;
use crate::CommandResult;

mod aggregate_eval;
mod create_transform;
mod function_eval_handler;
mod map_filter_eval;
mod parse;

#[derive(Args, Debug)]
pub struct Run {
    #[command(subcommand)]
    pub run: RunSubcommand,
}

#[derive(Clone, Debug)]
pub struct RunSubcommand {
    name: String,
}

pub async fn run_transform_command(command: &RunSubcommand, state: Arc<CliState>) -> CommandResult {
    let verbose = state.verbose;
    let client = state.client.clone();
    let config = state.require_config()?;

    let RunSubcommand { name } = command;

    let transform_config = config
        .transform_by_name(name.as_str())
        .ok_or_else(|| CommandError::Message(format!("No transform with name \"{name}\"")))?;

    let TransformConfig {
        name: _,
        source: from_collection_name,
        intermediate,
        target,
        reader_name,
        map_filter: _,
        aggregate: _,
        percentiles,
        unique_count,
    } = transform_config;

    if let Some(_) = intermediate {
        return Err(CommandError::Message(
            "Transforms with intermediate collection are not supported yet".to_string(),
        ));
    }
    if let Some(_) = percentiles {
        return Err(CommandError::Message(
            "Percentiles transforms are not supported yet".to_string(),
        ));
    }
    if let Some(_) = unique_count {
        return Err(CommandError::Message(
            "Unique count transforms are not supported yet".to_string(),
        ));
    }

    let to_collection_name = target.deref();
    let reader_name = reader_name.as_ref().map(|x| x.deref());

    let Some(reader_name) = reader_name else {
        return Err(CommandError::Message("No reader_name present".to_string()));
    };

    if let Some(to_collection_from_collections) = config.collection_by_name(to_collection_name) {
        let Collection {
            name: _,
            manual,
            human_readable: _,
        } = to_collection_from_collections;

        if !manual {
            return Err(CommandError::Message(format!(
                "Collection \"{to_collection_name}\" is not manual"
            )));
        }
    }

    let from_collection_name = from_collection_name.deref();

    let transform_direction = TransformDirection {
        from_collection_name,
        to_collection_name,
        reader_name,
    };

    let TransformEvaluator {
        transform,
        eval_handler,
    } = create_transform(config, transform_config, transform_direction, verbose).await?;

    // TODO: thread pool, parallelize function evals and diffbelt calls
    //       (they parse/serialize jsons currently)

    let mut inputs = Vec::new();
    let (sender, mut receiver) = mpsc::channel::<Result<Input, CommandError>>(8);

    let transform = Wrap::wrap(transform);
    let eval_handler = Wrap::wrap(eval_handler);

    let local = LocalSet::new();

    () = local
        .run_until(async {
            loop {
                let need_continue = processing_iteration(
                    &transform,
                    &eval_handler,
                    &mut inputs,
                    &sender,
                    &mut receiver,
                    &client,
                    verbose,
                )
                .await?;

                if !need_continue {
                    break;
                }
            }

            Ok::<(), CommandError>(())
        })
        .await?;

    println!("Finished");

    Ok(())
}

pub struct InputEmitter {
    action_id: (u64, u64),
    sender: mpsc::Sender<Result<Input, CommandError>>,
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

        () = loop {
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
    sender: &mpsc::Sender<Result<Input, CommandError>>,
    receiver: &mut mpsc::Receiver<Result<Input, CommandError>>,
    client: &Arc<DiffbeltClient>,
    verbose: bool,
) -> Result<bool, CommandError> {
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
                            let verbose_call_path = if verbose {
                                println!("> {action_id:?} db call {}", call.path);
                                Some(call.path.clone())
                            } else {
                                None
                            };

                            let message = match client.transform_call(call).await {
                                Ok(body) => {
                                    if verbose {
                                        println!(
                                            "< {action_id:?} db call {}",
                                            verbose_call_path
                                                .as_ref()
                                                .map(|x| x.as_ref())
                                                .unwrap_or("?")
                                        );
                                    }

                                    Ok(Input {
                                        id: action_id,
                                        input: InputType::DiffbeltCall(DiffbeltCallInput { body }),
                                    })
                                }
                                Err(err) => Err(err.into()),
                            };

                            () = sender.send(message).await.unwrap_or(());
                        });
                    }
                    ActionType::FunctionEval(eval) => {
                        let eval_handler = eval_handler.clone();
                        let transform = transform.clone();
                        spawn_local(async move {
                            () = eval_handler
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

    () = receive_inputs(inputs, receiver).await?;

    Ok(true)
}

async fn receive_inputs(
    inputs: &mut Vec<Input>,
    receiver: &mut mpsc::Receiver<Result<Input, CommandError>>,
) -> Result<(), CommandError> {
    if !inputs.is_empty() {
        return Ok(());
    }

    let input = receiver.recv().await;

    let Some(input) = input else {
        return Err(CommandError::Message(
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
