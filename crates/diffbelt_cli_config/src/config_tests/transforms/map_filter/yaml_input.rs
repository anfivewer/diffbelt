use crate::call_human_readable_conversion;
use diffbelt_protos::protos::transform::map_filter::{
    MapFilterInput, MapFilterInputArgs, MapFilterMultiInput, MapFilterMultiInputArgs,
};
use diffbelt_protos::{OwnedSerialized, Serializer};
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::value::parse_scalar;
use crate::wasm::human_readable::HumanReadableFunctions;

pub async fn yaml_test_vars_to_map_filter_input(
    human_readable_functions: &HumanReadableFunctions<'_>,
    node: &YamlNode,
) -> Result<OwnedSerialized<'static, MapFilterMultiInput<'static>>, YamlTestVarsError> {
    let mut serializer = Serializer::new();

    let map = node
        .as_mapping()
        .ok_or_else(|| YamlTestVarsError::Unspecified("vars should be a mapping".to_string()))?;

    let mut source_key_offset = None;
    let source_old_value_offset = None;
    let source_new_value_offset = None;

    let instance = human_readable_functions.instance;

    let input_vec_holder = instance.alloc_vec_holder().await?;
    let output_vec_holder = instance.alloc_vec_holder().await?;

    for (key, value) in map {
        let key = key.as_str().ok_or_else(|| {
            YamlTestVarsError::Unspecified("vars key should be string".to_string())
        })?;

        match key {
            "source_key" => {
                if let Some(s) = parse_scalar(value)?.as_str() {
                    () = call_human_readable_conversion!(
                        s.as_bytes(),
                        human_readable_functions,
                        call_key_to_bytes,
                        input_vec_holder,
                        output_vec_holder
                    )
                    .observe_bytes(instance, |bytes| {
                        source_key_offset = Some(serializer.create_vector(bytes));

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }
            }
            "source_old_value" => {
                if let Some(s) = parse_scalar(value)?.as_str() {
                    () = call_human_readable_conversion!(
                        s.as_bytes(),
                        human_readable_functions,
                        call_value_to_bytes,
                        input_vec_holder,
                        output_vec_holder
                    )
                    .observe_bytes(instance, |bytes| {
                        source_key_offset = Some(serializer.create_vector(bytes));

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }
            }
            "source_new_value" => {
                if let Some(s) = parse_scalar(value)?.as_str() {
                    () = call_human_readable_conversion!(
                        s.as_bytes(),
                        human_readable_functions,
                        call_value_to_bytes,
                        input_vec_holder,
                        output_vec_holder
                    )
                    .observe_bytes(instance, |bytes| {
                        source_key_offset = Some(serializer.create_vector(bytes));

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
