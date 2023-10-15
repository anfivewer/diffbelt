use crate::commands::errors::CommandError;
use diffbelt_cli_config::interpreter::function::Function;
use diffbelt_cli_config::interpreter::value::{Value, ValueHolder};
use diffbelt_cli_config::interpreter::var::{Var, VarDef};
use diffbelt_cli_config::CollectionValueFormat;
use diffbelt_transforms::base::action::function_eval::MapFilterEvalAction;
use diffbelt_transforms::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use diffbelt_transforms::base::input::{Input, InputType};
use diffbelt_util::option::lift_result_from_option;
use std::ops::Deref;
use std::rc::Rc;

pub struct MapFilterEvalOptions<'a> {
    pub action: MapFilterEvalAction,
    pub from_collection_format: CollectionValueFormat,
    pub to_collection_format: CollectionValueFormat,
    pub map_filter: &'a Function,
    pub inputs: &'a mut Vec<Input>,
    pub action_id: (u64, u64),
    pub map_filter_key_var_name: Rc<str>,
    pub map_filter_new_value_var_name: Rc<str>,
}

impl MapFilterEvalOptions<'_> {
    pub fn call(self) -> Result<(), CommandError> {
        let MapFilterEvalOptions {
            action,
            from_collection_format,
            to_collection_format,
            map_filter,
            inputs,
            action_id,
            map_filter_key_var_name,
            map_filter_new_value_var_name,
        } = self;

        let MapFilterEvalAction {
            key,
            from_value: _,
            to_value,
        } = action;

        // TODO: allow non-string keys
        let key = key.into_vec();
        let key = String::from_utf8(key).map_err(|_| {
            CommandError::Message("Non-utf8 keys are not yet supported".to_string())
        })?;
        let key = Rc::<str>::from(key);

        let to_value = to_value.map(|to_value| {
            let var = from_collection_format.boxed_bytes_to_var(to_value)?;
            Ok::<_, CommandError>(var)
        });
        let to_value = lift_result_from_option(to_value)?.unwrap_or_else(|| Var::new_none());

        let input_vars = vec![
            (
                map_filter_key_var_name,
                Var::new_string(key),
            ),
            (
                map_filter_new_value_var_name,
                to_value,
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
                        "Unexpected numeric return in map_filter function".to_string(),
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
                        return Err(CommandError::Message(format!(
                            "Unexpected return \"{unknown}\" key in map_filter function"
                        )));
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
                    body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                        old_key: Some(Box::from(target_key.as_bytes())),
                        new_key: None,
                        value: None,
                    }),
                }),
            });

            return Ok(());
        }

        let value = to_collection_format.value_to_boxed_bytes(target_value)?;

        inputs.push(Input {
            id: action_id,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    old_key: Some(Box::from(target_key.as_bytes())),
                    new_key: Some(Box::from(target_key.as_bytes())),
                    value: Some(value),
                }),
            }),
        });

        Ok(())
    }
}
