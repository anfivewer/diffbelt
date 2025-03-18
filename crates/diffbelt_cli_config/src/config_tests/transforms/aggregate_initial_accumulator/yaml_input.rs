use diffbelt_protos::protos::impls::AggregateTargetInfoProto;
use diffbelt_protos::protos::transform::aggregate::{AggregateTargetInfo, AggregateTargetInfoArgs};
use diffbelt_protos::{OwnedSerialized, Serializer};
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::value::{parse_scalar, Scalar};
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::{
    call_human_readable_conversion, yaml_test_vars_input_optional, yaml_test_vars_input_required,
};

pub async fn yaml_test_vars_to_aggregate_initial_accumulator_input(
    target_human_readable: &HumanReadableFunctions<'_>,
    node: &YamlNode,
) -> Result<OwnedSerialized<AggregateTargetInfoProto>, YamlTestVarsError> {
    let mut serializer = Serializer::new();

    let instance = target_human_readable.instance;
    let input_vec_holder = instance.alloc_vec_holder().await?;
    let output_vec_holder = instance.alloc_vec_holder().await?;

    let input = node
        .as_mapping()
        .ok_or_else(|| YamlTestVarsError::Unspecified("input should be a mapping".to_string()))?;

    let mut target_key_offset = None;
    let mut target_old_value_offset = None;

    for (key, value) in input {
        let key = key.as_str().ok_or_else(|| {
            YamlTestVarsError::Unspecified("mapping key should be a string".to_string())
        })?;

        let value = parse_scalar(value)?;

        match key {
            "target_key" => {
                yaml_test_vars_input_required!(
                    name: "target_key",
                    value: value.as_str(),
                    serializer,
                    human_readable: target_human_readable.call_key_to_bytes,
                    input_vec_holder,
                    output_vec_holder,
                    output_offset: target_key_offset,
                );
            }
            "target_old_value" => {
                yaml_test_vars_input_optional!(
                    value,
                    serializer,
                    human_readable: target_human_readable.call_value_to_bytes,
                    input_vec_holder,
                    output_vec_holder,
                    output_offset: target_old_value_offset,
                );
            }
            _ => {
                return Err(YamlTestVarsError::Unspecified(format!(
                    "unknown input key: {key}"
                )));
            }
        }
    }

    let current_generation_id = serializer.create_vector("current".as_bytes());
    let next_generation_id = serializer.create_vector("next".as_bytes());

    let input = AggregateTargetInfo::create(
        serializer.buffer_builder(),
        &AggregateTargetInfoArgs {
            target_key: target_key_offset,
            target_old_value: target_old_value_offset,
            generation_id: Some(current_generation_id),
            new_generation_id: Some(next_generation_id),
        },
    );

    Ok(serializer.finish(input).into_owned())
}
