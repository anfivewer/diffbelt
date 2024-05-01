use diffbelt_yaml::YamlNode;

use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::transforms::aggregate_util::yaml_node_to_aggregate_accumulator;
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
        let accumulator = yaml_node_to_aggregate_accumulator(
            aggregate_human_readable,
            item,
            &input_vec_holder,
            &output_vec_holder,
        )
        .await?;

        accumulators.push(accumulator);
    }

    Ok(accumulators)
}
