use diffbelt_cli_config::requests::client_impl::DiffbeltRequestsClientImpl;
use diffbelt_cli_config::requests::DiffbeltRequests;
use diffbelt_cli_config::transforms::aggregate::Aggregate;
use diffbelt_cli_config::transforms::wasm::WasmMethodDef;
use diffbelt_cli_config::transforms::Transform as TransformConfig;
use diffbelt_cli_config::wasm::engine::WasmEngine;
use diffbelt_cli_config::wasm::{NewWasmInstanceOptions, WasmModuleInstance};
use diffbelt_cli_config::CliConfig;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_transforms::aggregate::AggregateTransform;
use diffbelt_transforms::map_filter::MapFilterTransform;
use diffbelt_transforms::TransformImpl;
use std::sync::Arc;

use crate::commands::errors::CommandError;
use crate::commands::transform::run::aggregate_eval::AggregateEvalHandler;
use crate::commands::transform::run::function_eval_handler::FunctionEvalHandlerImpl;
use crate::commands::transform::run::map_filter_eval::MapFilterEvalHandler;

pub struct TransformEvaluator {
    // TODO: replace with enum_dispatch?
    pub transform: TransformImpl,
    pub eval_handler: FunctionEvalHandlerImpl,
    pub requests_impl: DiffbeltRequestsClientImpl,
}

pub struct TransformDirection<'a> {
    pub from_collection_name: &'a str,
    pub to_collection_name: &'a str,
    pub reader_name: &'a str,
}

pub async fn create_transform(
    engine: &mut WasmEngine,
    client: Arc<DiffbeltClient>,
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

    let (requests, requests_impl) = DiffbeltRequestsClientImpl::new(client);

    if let Some(map_filter_wasm) = map_filter_wasm {
        return create_map_filter_transform(
            engine,
            requests,
            requests_impl,
            map_filter_wasm,
            transform_direction,
            verbose,
        )
        .await;
    }

    if let Some(aggregate) = aggregate {
        return create_aggregate_transform(
            engine,
            requests,
            requests_impl,
            aggregate,
            transform_direction,
            verbose,
        )
        .await;
    }

    Err(CommandError::Message(
        "There should be at least one transform".to_string(),
    ))
}

async fn create_map_filter_transform(
    engine: &mut WasmEngine,
    requests: DiffbeltRequests,
    requests_impl: DiffbeltRequestsClientImpl,
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
    let wasm_module = engine.get_module(wasm_module_name).await?;

    let wasm_instance = WasmModuleInstance::new(NewWasmInstanceOptions {
        engine,
        module: &wasm_module,
        requests: Arc::new(requests),
    })
    .await?;

    let handler =
        MapFilterEvalHandler::new(wasm_instance, map_filter_wasm.method_name.as_str(), verbose)
            .await?;

    Ok(TransformEvaluator {
        transform: TransformImpl::MapFilter(transform),
        eval_handler: FunctionEvalHandlerImpl::MapFilter(handler),
        requests_impl,
    })
}

async fn create_aggregate_transform(
    engine: &mut WasmEngine,
    requests: DiffbeltRequests,
    requests_impl: DiffbeltRequestsClientImpl,
    aggregate: &Aggregate,
    transform_direction: TransformDirection<'_>,
    verbose: bool,
) -> Result<TransformEvaluator, CommandError> {
    let supports_accumulators_merge = aggregate.merge_accumulators.is_some();

    let transform = AggregateTransform::new(
        Box::from(transform_direction.from_collection_name),
        Box::from(transform_direction.to_collection_name),
        Box::from(transform_direction.reader_name),
        supports_accumulators_merge,
    );

    let wasm_module_name = aggregate.wasm.as_str();
    let wasm_module = engine.get_module(wasm_module_name).await?;

    let wasm_instance = WasmModuleInstance::new(NewWasmInstanceOptions {
        engine,
        module: &wasm_module,
        requests: Arc::new(requests),
    })
    .await?;

    let handler = AggregateEvalHandler::new(wasm_instance, aggregate, verbose).await?;

    Ok(TransformEvaluator {
        transform: TransformImpl::Aggregate(transform),
        eval_handler: FunctionEvalHandlerImpl::Aggregate(handler),
        requests_impl,
    })
}
