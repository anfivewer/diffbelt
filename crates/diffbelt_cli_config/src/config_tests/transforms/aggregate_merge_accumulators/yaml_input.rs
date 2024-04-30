use diffbelt_yaml::YamlNode;

use crate::call_human_readable_conversion;
use crate::config_tests::error::YamlTestVarsError;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;

pub async fn yaml_test_vars_to_aggregate_merge_accumulators_input(
    aggregate_human_readable: &AggregateHumanReadableFunctions<'_>,
    node: &YamlNode,
) -> Result<Vec<Vec<u8>>, YamlTestVarsError> {
    let instance = aggregate_human_readable.instance;
    let input_vec_holder = instance.alloc_vec_holder().await?;
    let output_vec_holder = instance.alloc_vec_holder().await?;

    let input = node
        .as_sequence()
        .ok_or_else(|| YamlTestVarsError::Unspecified("input should be a sequence".to_string()))?;

    let mut accumulators = Vec::with_capacity(input.items.len());

    for item in input {
        let item = item.as_str().ok_or_else(|| {
            YamlTestVarsError::Unspecified("input item should be a string".to_string())
        })?;

        () = call_human_readable_conversion!(
            item.as_bytes(),
            aggregate_human_readable,
            call_accumulator_to_bytes,
            input_vec_holder,
            output_vec_holder
        )
        .observe_bytes(instance, |bytes| {
            let accumulator = Vec::from(bytes);

            accumulators.push(accumulator);

            Ok::<_, YamlTestVarsError>(())
        })?;
    }

    Ok(accumulators)
}
