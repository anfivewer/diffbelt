use diffbelt_protos::protos::transform::aggregate::{
    AggregateReduceInput, AggregateReduceInputArgs, AggregateReduceItem, AggregateReduceItemArgs,
};
use diffbelt_protos::{OwnedSerialized, Serializer};
use diffbelt_yaml::YamlNode;

use crate::call_human_readable_conversion;
use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::value::parse_scalar;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;

pub async fn yaml_test_vars_to_aggregate_reduce_input(
    aggregate_human_readable: &AggregateHumanReadableFunctions<'_>,
    node: &YamlNode,
) -> Result<
    (
        Vec<u8>,
        OwnedSerialized<'static, AggregateReduceInput<'static>>,
    ),
    YamlTestVarsError,
> {
    let mut serializer = Serializer::new();

    let instance = aggregate_human_readable.instance;
    let input_vec_holder = instance.alloc_vec_holder().await?;
    let output_vec_holder = instance.alloc_vec_holder().await?;

    let input = node
        .as_mapping()
        .ok_or_else(|| YamlTestVarsError::Unspecified("input should be a mapping".to_string()))?;

    let mut accumulator = None;
    let mut items_wip = None;

    for (key, value) in input {
        let key = key.as_str().ok_or_else(|| {
            YamlTestVarsError::Unspecified("mapping key should be a string".to_string())
        })?;

        match key {
            "accumulator" => {
                let value = parse_scalar(value)?;
                let value = value.as_str().ok_or_else(|| {
                    YamlTestVarsError::Unspecified("accumulator is not a string".to_string())
                })?;

                () = call_human_readable_conversion!(
                    value.as_bytes(),
                    aggregate_human_readable,
                    call_accumulator_to_bytes,
                    input_vec_holder,
                    output_vec_holder
                )
                .observe_bytes(instance, |bytes| {
                    accumulator = Some(Vec::from(bytes));

                    Ok::<_, YamlTestVarsError>(())
                })?;
            }
            "items" => {
                let items_seq = value.as_sequence().ok_or_else(|| {
                    YamlTestVarsError::Unspecified("items is not a sequence".to_string())
                })?;

                let mut items = Vec::with_capacity(items_seq.items.len());

                for item in items_seq {
                    let item = item.as_str().ok_or_else(|| {
                        YamlTestVarsError::Unspecified("item is not a string".to_string())
                    })?;

                    () = call_human_readable_conversion!(
                        item.as_bytes(),
                        aggregate_human_readable,
                        call_mapped_value_to_bytes,
                        input_vec_holder,
                        output_vec_holder
                    )
                    .observe_bytes(instance, |bytes| {
                        let mapped_value = serializer.create_vector(bytes);

                        let item = AggregateReduceItem::create(
                            serializer.buffer_builder(),
                            &AggregateReduceItemArgs {
                                mapped_value: Some(mapped_value),
                            },
                        );

                        items.push(item);

                        Ok::<_, YamlTestVarsError>(())
                    })?;
                }

                items_wip = Some(items);
            }
            _ => {
                return Err(YamlTestVarsError::Unspecified(format!(
                    "unknown input key: {key}"
                )));
            }
        }
    }

    let accumulator =
        accumulator.ok_or_else(|| YamlTestVarsError::Unspecified("no accumulator".to_string()))?;
    let items = items_wip.ok_or_else(|| YamlTestVarsError::Unspecified("no items".to_string()))?;

    let items = serializer.create_vector(&items);

    let input = AggregateReduceInput::create(
        serializer.buffer_builder(),
        &AggregateReduceInputArgs { items: Some(items) },
    );

    Ok((accumulator, serializer.finish(input).into_owned()))
}
