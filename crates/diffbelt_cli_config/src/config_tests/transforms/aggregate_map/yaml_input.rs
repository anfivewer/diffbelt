use crate::call_human_readable_conversion;
use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiInput, AggregateMapMultiInputArgs, AggregateMapSource, AggregateMapSourceArgs,
    AggregateMapSourceBuilder,
};
use diffbelt_protos::{OwnedSerialized, Serializer, WIPOffset};
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::value::{parse_scalar, Scalar};
use crate::wasm::human_readable::HumanReadableFunctions;

pub fn yaml_test_vars_to_aggregate_map_input(
    source_human_readable: &HumanReadableFunctions,
    node: &YamlNode,
) -> Result<OwnedSerialized<'static, AggregateMapMultiInput<'static>>, YamlTestVarsError> {
    let mut serializer = Serializer::new();

    let instance = source_human_readable.instance;
    let input_vec_holder = instance.alloc_vec_holder()?;
    let output_vec_holder = instance.alloc_vec_holder()?;

    let source_items = node
        .as_sequence()
        .ok_or_else(|| YamlTestVarsError::Unspecified("input should be a sequence".to_string()))?;

    let source_items_len = source_items.items.len();
    serializer.start_vector::<WIPOffset<AggregateMapMultiInput>>(source_items_len);

    for source_item in source_items {
        let source_item = source_item.as_mapping().ok_or_else(|| {
            YamlTestVarsError::Unspecified("input items should be a mapping".to_string())
        })?;

        let mut source_key_offset = None;
        let mut source_old_value_offset = None;
        let mut source_new_value_offset = None;

        for (key, value) in source_item {
            let key = key.as_str().ok_or_else(|| {
                YamlTestVarsError::Unspecified("mapping key should be a string".to_string())
            })?;

            let value = parse_scalar(value)?;

            match key {
                "source_key" => {
                    let value = value.as_str().ok_or_else(|| {
                        YamlTestVarsError::Unspecified("source_key should be a string".to_string())
                    })?;

                    () = call_human_readable_conversion!(
                        value.as_bytes(),
                        source_human_readable,
                        call_key_to_bytes,
                        input_vec_holder,
                        output_vec_holder
                    )
                    .observe_bytes(instance, |bytes| {
                        source_key_offset = Some(serializer.create_vector(bytes));

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }
                "source_old_value" => {
                    if let Scalar::String(value) = value {
                        () = call_human_readable_conversion!(
                            value.as_bytes(),
                            source_human_readable,
                            call_value_to_bytes,
                            input_vec_holder,
                            output_vec_holder
                        )
                        .observe_bytes(instance, |bytes| {
                            source_old_value_offset = Some(serializer.create_vector(bytes));

                            Ok::<_, YamlTestVarsError>(())
                        })?;
                    }
                }
                "source_new_value" => {
                    if let Scalar::String(value) = value {
                        () = call_human_readable_conversion!(
                            value.as_bytes(),
                            source_human_readable,
                            call_value_to_bytes,
                            input_vec_holder,
                            output_vec_holder
                        )
                        .observe_bytes(instance, |bytes| {
                            source_new_value_offset = Some(serializer.create_vector(bytes));

                            Ok::<_, YamlTestVarsError>(())
                        })?;
                    }
                }
                _ => {
                    return Err(YamlTestVarsError::Unspecified(format!(
                        "unknown source item key: {key}"
                    )));
                }
            }
        }

        if source_key_offset.is_none() {
            return Err(YamlTestVarsError::Unspecified(
                "missing source key".to_string(),
            ));
        }

        let item = AggregateMapSource::create(
            serializer.buffer_builder(),
            &AggregateMapSourceArgs {
                source_key: source_key_offset,
                source_old_value: source_old_value_offset,
                source_new_value: source_new_value_offset,
            },
        );

        serializer.push(item);
    }

    let items = serializer.end_vector(source_items_len);

    let input = AggregateMapMultiInput::create(
        serializer.buffer_builder(),
        &AggregateMapMultiInputArgs { items: Some(items) },
    );

    Ok(serializer.finish(input).into_owned())
}
