use std::borrow::Cow;

use diffbelt_util::option::lift_result_from_option;
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::{TestError, YamlTestVarsError};
use crate::transforms::aggregate::{Aggregate, AggregateHumanReadable};
use crate::wasm::aggregate::AggregateFunctions;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::WasmModuleInstance;
use crate::{call_human_readable_conversion, Collection};

async fn create_aggregate_human_readable<'a>(
    human_readable: &'a WasmModuleInstance,
    aggregate_human_readable: &AggregateHumanReadable,
) -> Result<AggregateHumanReadableFunctions<'a>, TestError> {
    let aggregate_human_readable = AggregateHumanReadableFunctions::new(
        human_readable,
        aggregate_human_readable.bytes_to_target_key.as_str(),
        aggregate_human_readable.bytes_to_mapped_value.as_str(),
        aggregate_human_readable.mapped_value_to_bytes.as_str(),
        aggregate_human_readable.bytes_to_accumulator.as_str(),
        aggregate_human_readable.accumulator_to_bytes.as_str(),
    )
    .await?;

    Ok(aggregate_human_readable)
}

pub(crate) fn require_wasm_modules_aggregate<'a>(
    source_collection: Option<&'a Collection>,
    target_collection: Option<&'a Collection>,
    data: &'a Aggregate,
) -> Result<Vec<Cow<'a, str>>, TestError> {
    let mut modules = Vec::with_capacity(4);

    if let Some(collection) = source_collection {
        let Some(source) = &collection.human_readable else {
            return Err(TestError::SourceHasNoHumanReadableFunctions);
        };

        modules.push(Cow::Borrowed(source.wasm.as_str()));
    };

    if let Some(collection) = target_collection {
        let Some(target) = &collection.human_readable else {
            return Err(TestError::TargetHasNoHumanReadableFunctions);
        };

        modules.push(Cow::Borrowed(target.wasm.as_str()));
    };

    let Some(aggregate_human_readable) = &data.human_readable else {
        return Err(TestError::AggregateHasNoHumanReadableFunctions);
    };

    modules.push(Cow::Borrowed(data.wasm.as_str()));
    modules.push(Cow::Borrowed(aggregate_human_readable.wasm.as_str()));

    Ok(modules)
}

pub(crate) async fn create_test_aggregate_functions<'a>(
    source_collection: Option<&Collection>,
    target_collection: Option<&Collection>,
    data: &Aggregate,
    wasm_modules: Vec<&'a WasmModuleInstance>,
) -> Result<
    (
        Option<HumanReadableFunctions<'a>>,
        Option<HumanReadableFunctions<'a>>,
        AggregateFunctions<'a>,
        AggregateHumanReadableFunctions<'a>,
    ),
    TestError,
> {
    let mut wasm_module_index = 0;

    let source = source_collection.map(|collection| {
        let index = wasm_module_index;
        wasm_module_index += 1;
        wasm_modules
            .get(index)
            .map(|wasm| {
                (
                    collection.human_readable.as_ref().expect("already checked"),
                    wasm,
                )
            })
            .ok_or_else(|| {
                TestError::Panic(format!(
                    "wasm_module has wrong size: {}",
                    wasm_modules.len()
                ))
            })
    });
    let source = lift_result_from_option(source)?;

    let target = target_collection.map(|collection| {
        let index = wasm_module_index;
        wasm_module_index += 1;
        wasm_modules
            .get(index)
            .map(|wasm| {
                (
                    collection.human_readable.as_ref().expect("already checked"),
                    wasm,
                )
            })
            .ok_or_else(|| {
                TestError::Panic(format!(
                    "wasm_module has wrong size: {}",
                    wasm_modules.len()
                ))
            })
    });
    let target = lift_result_from_option(target)?;

    let aggregate_wasm = {
        let index = wasm_module_index;
        wasm_module_index += 1;
        wasm_modules.get(index).ok_or_else(|| {
            TestError::Panic(format!(
                "wasm_module has wrong size: {}",
                wasm_modules.len()
            ))
        })?
    };

    let aggregate_human_readable_wasm = {
        let index = wasm_module_index;
        wasm_modules.get(index).ok_or_else(|| {
            TestError::Panic(format!(
                "wasm_module has wrong size: {}",
                wasm_modules.len()
            ))
        })?
    };

    let source_human_readable = match source {
        None => None,
        Some((human_readable_def, wasm)) => {
            let human_readable = wasm
                .human_readable_functions(
                    human_readable_def.key_to_bytes.as_str(),
                    human_readable_def.bytes_to_key.as_str(),
                    human_readable_def.value_to_bytes.as_str(),
                    human_readable_def.bytes_to_value.as_str(),
                )
                .await?;

            Some(human_readable)
        }
    };

    let target_human_readable = match target {
        None => None,
        Some((human_readable_def, wasm)) => {
            let human_readable = wasm
                .human_readable_functions(
                    human_readable_def.key_to_bytes.as_str(),
                    human_readable_def.bytes_to_key.as_str(),
                    human_readable_def.value_to_bytes.as_str(),
                    human_readable_def.bytes_to_value.as_str(),
                )
                .await?;

            Some(human_readable)
        }
    };

    let aggregate_human_readable = data.human_readable.as_ref().expect("already checked");
    let aggregate_human_readable =
        create_aggregate_human_readable(aggregate_human_readable_wasm, aggregate_human_readable)
            .await?;

    let aggregate = AggregateFunctions::new(
        aggregate_wasm,
        data.map.as_str(),
        data.initial_accumulator.as_str(),
        data.reduce.as_str(),
        data.merge_accumulators.as_ref().map(|x| x.as_str()),
        data.apply.as_str(),
    )
    .await?;

    Ok((
        source_human_readable,
        target_human_readable,
        aggregate,
        aggregate_human_readable,
    ))
}

pub async fn yaml_node_to_aggregate_accumulator(
    aggregate_human_readable: &AggregateHumanReadableFunctions<'_>,
    node: &YamlNode,
    input_vec_holder: &WasmVecHolder<'_>,
    output_vec_holder: &WasmVecHolder<'_>,
) -> Result<Vec<u8>, YamlTestVarsError> {
    let instance = aggregate_human_readable.instance;

    let mut accumulator = None;

    let item = node.as_str().ok_or_else(|| {
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
        accumulator = Some(Vec::from(bytes));

        Ok::<_, YamlTestVarsError>(())
    })?;

    let accumulator = accumulator.expect("should be present");

    Ok(accumulator)
}
