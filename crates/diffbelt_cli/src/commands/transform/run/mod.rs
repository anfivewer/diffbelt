use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use clap::Args;
use tokio::pin;
use tokio::sync::mpsc;
use tokio::time::sleep;

use diffbelt_cli_config::interpreter::function::Function;
use diffbelt_cli_config::interpreter::value::{Value, ValueHolder};
use diffbelt_cli_config::interpreter::var::{Var, VarDef};
use diffbelt_cli_config::transforms::{
    CollectionDef, CollectionWithFormat, CollectionWithReader, Transform, TransformCollectionDef,
};
use diffbelt_cli_config::{Collection, CollectionValueFormat};
use diffbelt_transforms::base::action::function_eval::{FunctionEvalAction, MapFilterEvalAction};
use diffbelt_transforms::base::action::{Action, ActionType};
use diffbelt_transforms::base::input::diffbelt_call::DiffbeltCallInput;
use diffbelt_transforms::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_transforms::map_filter::MapFilterTransform;
use diffbelt_transforms::TransformRunResult;
use diffbelt_util::option::lift_result_from_option;

use crate::commands::errors::CommandError;
use crate::state::CliState;
use crate::CommandResult;

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
    let client = state.client.clone();
    let config = state.require_config()?;

    let RunSubcommand { name } = command;

    let transform_config = config
        .transform_by_name(name.as_str())
        .ok_or_else(|| CommandError::Message(format!("No transform with name \"{name}\"")))?;

    let Transform {
        name: _,
        from: from_collection_name,
        intermediate,
        to,
        reader_name,
        map_filter,
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

    struct CollectionInfoFromTransform<'a> {
        name: &'a str,
        reader_name: Option<&'a str>,
        format: Option<CollectionValueFormat>,
    }

    let CollectionInfoFromTransform {
        name: to_collection_name,
        reader_name: to_collection_reader_name,
        format: mut to_collection_format,
    } = match to {
        TransformCollectionDef::Named(name) => CollectionInfoFromTransform {
            name: name.as_str(),
            reader_name: None,
            format: None,
        },
        TransformCollectionDef::WithReader(with_reader) => {
            let CollectionWithReader {
                collection,
                reader_name,
            } = with_reader;
            match collection {
                CollectionDef::Named(name) => CollectionInfoFromTransform {
                    name: name.as_str(),
                    reader_name: Some(reader_name.as_str()),
                    format: None,
                },
                CollectionDef::WithFormat(with_format) => {
                    let CollectionWithFormat { name, format } = with_format;

                    let Some(format) = CollectionValueFormat::from_str(format.as_str()) else {
                        return Err(CommandError::Message(format!(
                            "Unknown collection values format \"{format}\""
                        )));
                    };

                    CollectionInfoFromTransform {
                        name: name.as_str(),
                        reader_name: Some(reader_name.as_str()),
                        format: Some(format),
                    }
                }
                CollectionDef::Unknown(node) => {
                    let mark = &node.start_mark;
                    let line = mark.line;
                    let column = mark.column;

                    return Err(CommandError::Message(format!(
                        "Unknown \"to.collection\" definition, line {line}:{column}"
                    )));
                }
            }
        }
        TransformCollectionDef::Unknown(node) => {
            let mark = &node.start_mark;
            let line = mark.line;
            let column = mark.column;

            return Err(CommandError::Message(format!(
                "Unknown \"to\" collection definition, line {line}:{column}"
            )));
        }
    };

    if let (Some(reader_name_a), Some(reader_name_b)) = (to_collection_reader_name, reader_name) {
        let reader_name_b = reader_name_b.deref();
        if reader_name_a != reader_name_b {
            return Err(CommandError::Message(format!(
                "Conflicting reader name, \"{reader_name_a}\" vs \"{reader_name_b}\""
            )));
        }
    }

    let reader_name =
        to_collection_reader_name.or_else(|| reader_name.as_ref().map(|name| name.deref()));

    let Some(reader_name) = reader_name else {
        return Err(CommandError::Message("No reader_name present".to_string()));
    };

    if let Some(to_collection_from_collections) = config.collection_by_name(to_collection_name) {
        let Collection {
            name: _,
            manual,
            format,
        } = to_collection_from_collections;

        if !manual {
            return Err(CommandError::Message(format!(
                "Collection \"{to_collection_name}\" is not manual"
            )));
        }

        if let Some(to_collection_format) = &to_collection_format {
            if to_collection_format != format {
                let to_collection_format = to_collection_format.as_str();
                let format = format.as_str();
                return Err(CommandError::Message(format!(
                    "Conflicting collection format, \"{to_collection_format}\" vs \"{format}\""
                )));
            }
        } else {
            to_collection_format.replace(format.clone());
        }
    }

    let Some(to_collection_format) = to_collection_format else {
        return Err(CommandError::Message(
            "No target collection format present".to_string(),
        ));
    };

    let from_collection_name = from_collection_name.deref();

    let Some(from_collection) = config.collection_by_name(from_collection_name) else {
        // TODO: collect all collections definitions around all config, not only from "collections" block
        return Err(CommandError::Message(format!(
            "No collection \"{from_collection_name}\" definition present"
        )));
    };

    let Collection {
        name: _,
        manual: _,
        format: from_collection_format,
    } = from_collection;

    if from_collection_format != &CollectionValueFormat::Utf8 {
        return Err(CommandError::Message(
            "Only utf8 source collection format supported yet".to_string(),
        ));
    }

    let Some(map_filter) = map_filter else {
        return Err(CommandError::Message("Unknown transform type".to_string()));
    };

    let map_filter_key_var_name = Rc::<str>::from("map_filter_key");
    let map_filter_new_value_var_name = Rc::<str>::from("map_filter_key");

    // TODO: extract this, currently present in multiple files
    let map_filter_input_vars = [
        (map_filter_key_var_name.clone(), VarDef::anonymous_string()),
        (
            map_filter_new_value_var_name.clone(),
            VarDef::anonymous_string(),
        ),
    ]
    .into_iter()
    .collect();

    let map_filter = Function::from_code(config, map_filter, Some(map_filter_input_vars))?;

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
        if inputs.is_empty() {
            let timeout_fut = sleep(Duration::from_millis(500));
            pin!(timeout_fut);

            loop {
                let is_timedout = tokio::select! {
                    input = receiver.recv() => {
                        let Some(input) = input else {
                            return Err(CommandError::Message("Inputs channel was closed".to_string()));
                        };

                        let input = input?;

                        inputs.push(input);
                        false
                    },
                    _ = &mut timeout_fut => {
                        true
                    },
                };

                if is_timedout {
                    break;
                }
            }

            if inputs.is_empty() {
                continue;
            }
        }

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
                                let message = match client.transform_call(call).await {
                                    Ok(body) => Ok(Input {
                                        id: action_id,
                                        input: InputType::DiffbeltCall(DiffbeltCallInput { body }),
                                    }),
                                    Err(err) => Err(err.into()),
                                };

                                () = sender.send(message).await.unwrap_or(());
                            });
                        }
                        ActionType::FunctionEval(eval) => match eval {
                            FunctionEvalAction::MapFilter(action) => {
                                let MapFilterEvalAction {
                                    key,
                                    from_value: _,
                                    to_value,
                                } = action;

                                // TODO: allow non-string keys
                                let key = key.into_vec();
                                let key = String::from_utf8(key).map_err(|_| {
                                    CommandError::Message(
                                        "Non-utf8 keys are not yet supported".to_string(),
                                    )
                                })?;
                                let key = Rc::<str>::from(key);

                                // TODO: support other source value formats
                                let to_value = to_value.map(|to_value| {
                                    let to_value = to_value.into_vec();
                                    let to_value = String::from_utf8(to_value).map_err(|_| {
                                        CommandError::Message(
                                            "Non-utf8 values are not yet supported".to_string(),
                                        )
                                    })?;
                                    let to_value = Rc::<str>::from(to_value);
                                    Ok::<_, CommandError>(to_value)
                                });
                                let to_value = lift_result_from_option(to_value)?;

                                if from_collection_format != &CollectionValueFormat::Utf8 {
                                    return Err(CommandError::Message(
                                        "Only utf8 source collection format supported yet"
                                            .to_string(),
                                    ));
                                }

                                let input_vars = vec![
                                    (
                                        map_filter_key_var_name.clone(),
                                        Var {
                                            def: VarDef::anonymous_string(),
                                            value: Some(ValueHolder {
                                                value: Value::String(key),
                                            }),
                                        },
                                    ),
                                    (
                                        map_filter_new_value_var_name.clone(),
                                        Var {
                                            def: VarDef::anonymous_string(),
                                            value: Some(ValueHolder {
                                                value: to_value
                                                    .map(|s| Value::String(s))
                                                    .unwrap_or(Value::None),
                                            }),
                                        },
                                    ),
                                ]
                                .into_iter()
                                .collect();

                                let map = map_filter.call(input_vars)?;

                                let mut target_key = None;
                                let mut target_value = None;

                                let Some(map) = map.as_map() else {
                                    return Err(CommandError::Message(
                                        "map_filter function returned not a map".to_string(),
                                    ));
                                };

                                {
                                    let map = map.borrow();
                                    let map = map.deref();

                                    for (key, value) in map.iter() {
                                        let Some(key) = key.as_str() else {
                                            return Err(CommandError::Message(
                                                "Unexpected numeric return in map_filter function"
                                                    .to_string(),
                                            ));
                                        };

                                        match key {
                                            "key" => {
                                                let Some(value) = value.as_rc_str() else {
                                                    return Err(CommandError::Message(
                                                        "Unexpected non-string target key in map_filter function return"
                                                            .to_string(),
                                                    ));
                                                };

                                                target_key = Some(value);
                                            }
                                            "value" => {
                                                target_value = Some(value.clone());
                                            }
                                            unknown => {
                                                return Err(CommandError::Message(
                                                    format!("Unexpected return \"{unknown}\" key in map_filter function"),
                                                ));
                                            }
                                        }
                                    }
                                }

                                let Some(target_key) = target_key else {
                                    return Err(CommandError::Message(
                                        "map_filter function did not returned key".to_string(),
                                    ));
                                };
                                let Some(target_value) = target_value else {
                                    return Err(CommandError::Message(
                                        "map_filter function did not returned value".to_string(),
                                    ));
                                };

                                if target_value.is_none() {
                                    inputs.push(Input {
                                        id: action_id,
                                        input: InputType::FunctionEval(FunctionEvalInput {
                                            body: FunctionEvalInputBody::MapFilter(
                                                MapFilterEvalInput {
                                                    old_key: Some(Box::from(target_key.as_bytes())),
                                                    new_key: None,
                                                    value: None,
                                                },
                                            ),
                                        }),
                                    });

                                    // FIXME: OMG, such nesting, what this continue continues?
                                    continue;
                                }

                                match to_collection_format {
                                    CollectionValueFormat::Bytes => {
                                        return Err(CommandError::Message(
                                            "Bytes format is not yet supported".to_string(),
                                        ));
                                    }
                                    CollectionValueFormat::Utf8 => {
                                        let Some(value) = target_value.as_str() else {
                                            return Err(CommandError::Message(
                                                "Target collection has utf8 format, but value returned from map_filter function is not a string".to_string(),
                                            ));
                                        };

                                        inputs.push(Input {
                                            id: action_id,
                                            input: InputType::FunctionEval(FunctionEvalInput {
                                                body: FunctionEvalInputBody::MapFilter(
                                                    MapFilterEvalInput {
                                                        old_key: Some(Box::from(
                                                            target_key.as_bytes(),
                                                        )),
                                                        new_key: Some(Box::from(
                                                            target_key.as_bytes(),
                                                        )),
                                                        value: Some(Box::from(value.as_bytes())),
                                                    },
                                                ),
                                            }),
                                        });
                                    }
                                    CollectionValueFormat::Json => {
                                        todo!("Serialize value to json")
                                    }
                                }
                            }
                        },
                    }
                }
            }
            TransformRunResult::Finish => {
                break;
            }
        }
    }

    println!("Finished");

    Ok(())
}
