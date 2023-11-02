use std::mem;
use std::ops::Deref;
use std::sync::Arc;

use clap::Args;
use tokio::sync::mpsc;

use diffbelt_cli_config::transforms::Transform;
use diffbelt_cli_config::Collection;
use diffbelt_transforms::base::action::function_eval::FunctionEvalAction;
use diffbelt_transforms::base::action::{Action, ActionType};
use diffbelt_transforms::base::input::diffbelt_call::DiffbeltCallInput;
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_transforms::map_filter::MapFilterTransform;
use diffbelt_transforms::TransformRunResult;

use crate::commands::errors::CommandError;
use crate::commands::transform::run::map_filter_eval::MapFilterEvalOptions;
use crate::state::CliState;
use crate::CommandResult;

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

    let Transform {
        name: _,
        source: from_collection_name,
        intermediate,
        target,
        reader_name,
        map_filter: map_filter_wasm,
        aggregate,
        percentiles,
        unique_count,
    } = transform_config;

    if let Some(_) = intermediate {
        return Err(CommandError::Message(
            "Transforms with intermediate collection are not supported yet".to_string(),
        ));
    }
    if let Some(_) = aggregate {
        return Err(CommandError::Message(
            "Aggregate transforms are not supported yet".to_string(),
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

    let Some(map_filter_wasm) = map_filter_wasm else {
        return Err(CommandError::Message("Unknown transform type".to_string()));
    };

    let wasm_module_name = map_filter_wasm.module_name.as_str();
    let Some(wasm_def) = config.wasm_module_def_by_name(wasm_module_name) else {
        return Err(CommandError::Message(format!(
            "WASM module {wasm_module_name} not defined in config"
        )));
    };

    let wasm_instance = config.new_wasm_instance(wasm_def).await?;
    let map_filter = wasm_instance.map_filter_function(map_filter_wasm.method_name.as_str())?;

    let mut transform = MapFilterTransform::new(
        Box::from(from_collection_name),
        Box::from(to_collection_name),
        Box::from(reader_name),
    );

    // TODO: thread pool, parallelize function evals and diffbelt calls
    //       (they parse/serialize jsons currently)

    let mut inputs = Vec::new();
    let (sender, mut receiver) = mpsc::channel::<Result<Input, CommandError>>(8);

    loop {
        let mut prev_inputs = Vec::new();
        mem::swap(&mut prev_inputs, &mut inputs);
        let run_result = transform.run(prev_inputs)?;

        match run_result {
            TransformRunResult::Actions(actions) => {
                for action in actions {
                    let Action {
                        id: action_id,
                        action,
                    } = action;

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
                                            input: InputType::DiffbeltCall(DiffbeltCallInput {
                                                body,
                                            }),
                                        })
                                    }
                                    Err(err) => Err(err.into()),
                                };

                                () = sender.send(message).await.unwrap_or(());
                            });
                        }
                        ActionType::FunctionEval(eval) => match eval {
                            FunctionEvalAction::MapFilter(action) => {
                                () = MapFilterEvalOptions {
                                    verbose,
                                    action,
                                    map_filter: &map_filter,
                                    inputs: &mut inputs,
                                    action_id,
                                }
                                .call()?;
                            }
                        },
                    }
                }
            }
            TransformRunResult::Finish => {
                break;
            }
        }

        () = receive_inputs(&mut inputs, &mut receiver).await?;
    }

    println!("Finished");

    Ok(())
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
