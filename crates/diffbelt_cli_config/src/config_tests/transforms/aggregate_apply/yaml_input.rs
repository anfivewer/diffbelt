use diffbelt_yaml::YamlNode;

use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::transforms::aggregate_util::yaml_node_to_aggregate_accumulator;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;

pub async fn yaml_test_vars_to_aggregate_apply_input(
    aggregate_human_readable: &AggregateHumanReadableFunctions<'_>,
    node: &YamlNode,
) -> Result<Vec<u8>, YamlTestVarsError> {
    let instance = aggregate_human_readable.instance;
    let input_vec_holder = instance.alloc_vec_holder().await?;
    let output_vec_holder = instance.alloc_vec_holder().await?;

    let accumulator = yaml_node_to_aggregate_accumulator(
        aggregate_human_readable,
        node,
        &input_vec_holder,
        &output_vec_holder,
    )
    .await?;

    Ok(accumulator)
}
