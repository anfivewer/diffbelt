use diffbelt_protos::protos::transform::map_filter::{
    MapFilterInput, MapFilterInputArgs, MapFilterMultiInput, MapFilterMultiInputArgs,
};
use diffbelt_protos::{OwnedSerialized, Serializer};
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::value::parse_scalar;
use crate::wasm::human_readable::HumanReadableFunctions;

pub fn yaml_test_vars_to_map_filter_input(
    human_readable_functions: &HumanReadableFunctions,
    node: &YamlNode,
) -> Result<OwnedSerialized<'static, MapFilterMultiInput<'static>>, YamlTestVarsError> {
    let mut serializer = Serializer::new();

    let map = node
        .as_mapping()
        .ok_or_else(|| YamlTestVarsError::Unspecified("vars should be a mapping".to_string()))?;

    let mut source_key_offset = None;
    let mut source_old_value_offset = None;
    let mut source_new_value_offset = None;

    let instance = human_readable_functions.instance;

    let input_vec_holder = instance.alloc_vec_holder()?;
    let output_vec_holder = instance.alloc_vec_holder()?;

    for (key, value) in map {
        let key = key.as_str().ok_or_else(|| {
            YamlTestVarsError::Unspecified("vars key should be string".to_string())
        })?;

        match key {
            "source_key" => {
                if let Some(s) = parse_scalar(value)?.as_str() {
                    () = instance.replace_vec_with_slice(&input_vec_holder, s.as_bytes())?;
                    let slice = instance.vec_to_bytes_slice(&input_vec_holder)?;

                    () =
                        human_readable_functions.call_bytes_to_key(&slice.0, &output_vec_holder)?;

                    let result = output_vec_holder.access()?;
                    () = result.observe_bytes(|bytes| {
                        source_key_offset = Some(serializer.create_vector(bytes));

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }
            }
            "source_old_value" => {
                if let Some(s) = parse_scalar(value)?.as_str() {
                    () = instance.replace_vec_with_slice(&input_vec_holder, s.as_bytes())?;
                    let slice = instance.vec_to_bytes_slice(&input_vec_holder)?;

                    () = human_readable_functions
                        .call_value_to_bytes(&slice.0, &output_vec_holder)?;

                    let result = output_vec_holder.access()?;
                    () = result.observe_bytes(|bytes| {
                        source_old_value_offset = Some(serializer.create_vector(bytes));

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }
            }
            "source_new_value" => {
                if let Some(s) = parse_scalar(value)?.as_str() {
                    () = instance.replace_vec_with_slice(&input_vec_holder, s.as_bytes())?;
                    let slice = instance.vec_to_bytes_slice(&input_vec_holder)?;

                    () = human_readable_functions
                        .call_value_to_bytes(&slice.0, &output_vec_holder)?;

                    let result = output_vec_holder.access()?;
                    () = result.observe_bytes(|bytes| {
                        source_new_value_offset = Some(serializer.create_vector(bytes));

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }
            }
            _ => {
                return Err(YamlTestVarsError::Unspecified(format!(
                    "unknown vars key: {key}"
                )));
            }
        }
    }

    let input = MapFilterInput::create(
        serializer.buffer_builder(),
        &MapFilterInputArgs {
            source_key: source_key_offset,
            source_old_value: source_old_value_offset,
            source_new_value: source_new_value_offset,
        },
    );

    let offset = serializer.create_vector(&[input]);

    let offset = MapFilterMultiInput::create(
        serializer.buffer_builder(),
        &MapFilterMultiInputArgs {
            items: Some(offset),
        },
    );

    return Ok(serializer.finish(offset).into_owned());
}
