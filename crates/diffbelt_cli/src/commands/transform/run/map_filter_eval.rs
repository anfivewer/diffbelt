use diffbelt_cli_config::wasm::MapFilterFunction;
use diffbelt_protos::{deserialize, OwnedSerialized};
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiOutput;
use diffbelt_transforms::base::action::function_eval::MapFilterEvalAction;
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_transforms::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use diffbelt_util::errors::NoStdErrorWrap;

use crate::commands::errors::MapFilterEvalError;

pub struct MapFilterEvalOptions<'a> {
    pub verbose: bool,
    pub action: MapFilterEvalAction,
    pub map_filter: &'a MapFilterFunction<'a>,
    pub inputs: &'a mut Vec<Input>,
    pub action_id: (u64, u64),
}

impl MapFilterEvalOptions<'_> {
    pub fn call(self) -> Result<(), MapFilterEvalError> {
        let MapFilterEvalOptions {
            verbose,
            action,
            map_filter,
            inputs,
            action_id,
        } = self;

        let MapFilterEvalAction {
            input,
            output_buffer: mut outputs_buffer,
        } = action;

        let map_filter_multi_input = input.data();

        if verbose {
            println!(
                "!> map_filter {action_id:?} {} records",
                map_filter_multi_input.items().map(|x| x.len()).unwrap_or(0)
            );
        }

        let output = map_filter.call(input.as_bytes())?;

        () = output.observe_bytes(|bytes| {
            let output = deserialize::<MapFilterMultiOutput>(bytes).map_err(NoStdErrorWrap)?;
            let Some(records) = output.target_update_records() else {
                return Err(MapFilterEvalError::Unspecified(
                    "map_filter function did not returned event empty target_update_records"
                        .to_string(),
                ));
            };

            if verbose {
                println!("!< map_filter {} records", records.len());
            }

            outputs_buffer.clear();
            outputs_buffer.extend_from_slice(bytes);

            Ok::<_, MapFilterEvalError>(())
        })?;

        let output = OwnedSerialized::<MapFilterMultiOutput<'static>>::from_vec(outputs_buffer)
            .map_err(NoStdErrorWrap)?;

        inputs.push(Input {
            id: action_id,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    input: output,
                    output_buffer: input.into_vec(),
                }),
            }),
        });

        Ok(())
    }
}
