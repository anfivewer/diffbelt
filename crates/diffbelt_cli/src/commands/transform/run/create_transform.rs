use diffbelt_cli_config::transforms::aggregate::Aggregate;
use diffbelt_cli_config::transforms::wasm::WasmMethodDef;
use diffbelt_cli_config::transforms::Transform as TransformConfig;
use diffbelt_cli_config::wasm::WasmModuleInstance;
use diffbelt_cli_config::CliConfig;
use diffbelt_transforms::map_filter::MapFilterTransform;
use diffbelt_transforms::TransformImpl;

use crate::commands::errors::CommandError;
use crate::commands::transform::run::function_eval_handler::FunctionEvalHandlerImpl;
use crate::commands::transform::run::map_filter_eval::MapFilterEvalHandler;

pub struct TransformEvaluator {
    // TODO: replace with enum_dispatch?
    pub transform: TransformImpl,
    pub eval_handler: FunctionEvalHandlerImpl,
}

pub struct TransformDirection<'a> {
    pub from_collection_name: &'a str,
    pub to_collection_name: &'a str,
    pub reader_name: &'a str,
}

pub async fn create_transform(
    config: &CliConfig,
    transform_config: &TransformConfig,
    transform_direction: TransformDirection<'_>,
    verbose: bool,
) -> Result<TransformEvaluator, CommandError> {
    let diffbelt_cli_config::transforms::Transform {
        name: _,
        source: _from_collection_name,
        intermediate,
        target: _,
        reader_name: _,
        map_filter: map_filter_wasm,
        aggregate,
        percentiles,
        unique_count,
    } = transform_config;

    let transform_types_count = map_filter_wasm.as_ref().map(|_| 1).unwrap_or(0)
        + aggregate.as_ref().map(|_| 1).unwrap_or(0)
        + percentiles.as_ref().map(|_| 1).unwrap_or(0)
        + unique_count.as_ref().map(|_| 1).unwrap_or(0);

    if transform_types_count != 1 {
        return Err(CommandError::Message(
            "Conflicting transforms specified".to_string(),
        ));
    }

    if let Some(_) = intermediate {
        return Err(CommandError::Message(
            "Transforms with intermediate collection are not supported yet".to_string(),
        ));
    }

    if let Some(_) = percentiles {
        return Err(CommandError::Message(
            "Percentiles transforms are not supported yet".to_string(),
        ));
    }
    if let Some(_) = unique_count {
        return Err(CommandError::Message(
            "Unique count transforms are not supported yet".to_string(),
        ));
    }

    if let Some(map_filter_wasm) = map_filter_wasm {
        return create_map_filter_transform(config, map_filter_wasm, transform_direction, verbose)
            .await;
    }

    if let Some(aggregate) = aggregate {
        return create_aggregate_transform(aggregate).await;
    }

    Err(CommandError::Message(
        "There should be at least one transform".to_string(),
    ))
}

async fn create_map_filter_transform(
    config: &CliConfig,
    map_filter_wasm: &WasmMethodDef,
    transform_direction: TransformDirection<'_>,
    verbose: bool,
) -> Result<TransformEvaluator, CommandError> {
    let transform = MapFilterTransform::new(
        Box::from(transform_direction.from_collection_name),
        Box::from(transform_direction.to_collection_name),
        Box::from(transform_direction.reader_name),
    );

    let wasm_module_name = map_filter_wasm.module_name.as_str();
    let Some(wasm_def) = config.wasm_module_def_by_name(wasm_module_name) else {
        return Err(CommandError::Message(format!(
            "WASM module {wasm_module_name} not defined in config"
        )));
    };

    let wasm_instance = config.new_wasm_instance(wasm_def).await?;
    let wasm_instance = Box::new(wasm_instance);
    let wasm_instance = Box::leak(wasm_instance);

    let map_filter = wasm_instance
        .map_filter_function(map_filter_wasm.method_name.as_str())
        .await?;
    let vec_holder = wasm_instance.alloc_vec_holder().await?;

    let handler = MapFilterEvalHandler {
        verbose,
        instance: wasm_instance as *const WasmModuleInstance,
        vec_holder,
        map_filter,
    };

    Ok(TransformEvaluator {
        transform: TransformImpl::MapFilter(transform),
        eval_handler: FunctionEvalHandlerImpl::MapFilter(handler),
    })
}

async fn create_aggregate_transform(
    _aggregate: &Aggregate,
) -> Result<TransformEvaluator, CommandError> {
    todo!()
}

//
