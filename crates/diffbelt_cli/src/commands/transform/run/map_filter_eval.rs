use std::ops::Deref;
use std::rc::Rc;
use thiserror::Error;

use diffbelt_cli_config::interpreter::function::Function;
use diffbelt_cli_config::interpreter::var::Var;
use diffbelt_cli_config::wasm::{MapFilterFunction, WasmError};
use diffbelt_protos::protos::transform::map_filter::{
    MapFilterInput, MapFilterInputArgs, MapFilterMultiInput, MapFilterMultiInputArgs,
    MapFilterMultiOutput,
};
use diffbelt_protos::{deserialize, InvalidFlatbuffer, Serializer};
use diffbelt_transforms::base::action::function_eval::MapFilterEvalAction;
use diffbelt_transforms::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::impl_from_either;

use crate::commands::errors::CommandError;

pub struct MapFilterEvalOptions<'a> {
    pub verbose: bool,
    pub actions: Vec<MapFilterEvalAction>,
    pub map_filter: &'a MapFilterFunction<'a>,
    pub inputs: &'a mut Vec<Input>,
    pub action_id: (u64, u64),
    pub buffer: Vec<u8>,
}

pub struct MapFilterEvalResult {
    pub buffer: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum MapFilterEvalError {
    #[error("{0}")]
    Unspecified(String),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    InvalidFlatbuffer(#[from] InvalidFlatbuffer),
}

impl_from_either!(MapFilterEvalError);

impl MapFilterEvalOptions<'_> {
    pub fn call(self) -> Result<MapFilterEvalResult, MapFilterEvalError> {
        let MapFilterEvalOptions {
            verbose,
            actions,
            map_filter,
            inputs,
            action_id,
            buffer,
        } = self;

        let mut serializer = Serializer::from_vec(buffer);

        let mut records = Vec::with_capacity(actions.len());

        for action in actions {
            let MapFilterEvalAction {
                source_key: key,
                source_old_value: from_value,
                source_new_value: to_value,
            } = action;

            let source_key = serializer.create_vector(key.deref());
            let source_old_value = from_value.map(|x| serializer.create_vector(x.deref()));
            let source_new_value = to_value.map(|x| serializer.create_vector(x.deref()));

            let item = MapFilterInput::create(
                serializer.buffer_builder(),
                &MapFilterInputArgs {
                    source_key: Some(source_key),
                    source_old_value,
                    source_new_value,
                },
            );

            records.push(item);
        }

        if verbose {
            println!("!> map_filter {action_id:?} {} records", records.len());
        }

        let records = serializer.create_vector(records.as_slice());
        let input = MapFilterMultiInput::create(
            serializer.buffer_builder(),
            &MapFilterMultiInputArgs {
                items: Some(records),
            },
        );
        let input = serializer.finish(input);

        let output = map_filter.call(input.data())?;

        let q = output.observe_bytes(|bytes| {
            let output = deserialize::<MapFilterMultiOutput>(bytes)?;
            let Some(records) = output.target_update_records() else {
                return Err(MapFilterEvalError::Unspecified(
                    "map_filter function did not returned event empty target_update_records"
                        .to_string(),
                ));
            };

            if verbose {
                println!("!< map_filter {} records", records.len());
            }

            for record in records {
                let key = record.key().ok_or_else(|| MapFilterEvalError::Unspecified(
                    "map_filter function returned not present RecordUpdate.key, it should be at least empty"
                        .to_string(),
                ));
                let value = record.value();

                // TODO: push results to inputs
            }

            Ok::<_, MapFilterEvalError>(())
        })?;

        // if target_value.is_none() {
        //     inputs.push(Input {
        //         id: action_id,
        //         input: InputType::FunctionEval(FunctionEvalInput {
        //             body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
        //                 old_key: Some(Box::from(target_key.as_bytes())),
        //                 new_key: None,
        //                 value: None,
        //             }),
        //         }),
        //     });
        //
        //     return Ok(());
        // }
        //
        // inputs.push(Input {
        //     id: action_id,
        //     input: InputType::FunctionEval(FunctionEvalInput {
        //         body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
        //             old_key: Some(Box::from(target_key.as_bytes())),
        //             new_key: Some(Box::from(target_key.as_bytes())),
        //             value: Some(value),
        //         }),
        //     }),
        // });

        Ok(MapFilterEvalResult {
            buffer: input.into_empty_vec(),
        })
    }
}
