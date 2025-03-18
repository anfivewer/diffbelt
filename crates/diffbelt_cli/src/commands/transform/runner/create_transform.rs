use diffbelt_cli_config::requests::client_impl::DiffbeltRequestsClientImpl;
use diffbelt_cli_config::requests::DiffbeltRequests;
use diffbelt_cli_config::transforms::aggregate::Aggregate;
use diffbelt_cli_config::transforms::wasm::WasmMethodDef;
use diffbelt_cli_config::wasm::engine::WasmEngine;
use diffbelt_cli_config::wasm::{NewWasmInstanceOptions, WasmModuleInstance};
use diffbelt_cli_config::CliConfig;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_transforms::aggregate::AggregateTransform;
use diffbelt_transforms::map_filter::MapFilterTransform;
use diffbelt_transforms::TransformImpl;
use std::sync::Arc;

use crate::commands::errors::CommandError;
use crate::commands::transform::runner::aggregate_eval::AggregateEvalHandler;
use crate::commands::transform::runner::function_eval_handler::FunctionEvalHandlerImpl;
use crate::commands::transform::runner::map_filter_eval::MapFilterEvalHandler;
use crate::commands::transform::runner::transform_config::{
    AggregateTransformInfo, MapFilterTransformInfo, TransformInfo, TransformTypeInfo,
};
use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::wasm::cli_api::WasmCliApi;

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
    engine: &WasmEngine,
    client: Arc<DiffbeltClient>,
    cli_api: WasmCliApi,
    transform_config: &TransformInfo,
    transform_direction: TransformDirection<'_>,
) -> Result<TransformEvaluator, RunTransformError> {
    let TransformInfo {
        source_collection_name,
        intermediate_collection_name,
        target_collection_name,
        reader_name,
        transform,
    } = transform_config;

    let (requests, requests_impl) = DiffbeltRequestsClientImpl::new(client);

    match transform {
        TransformTypeInfo::MapFilter(info) => {
            create_map_filter_transform(
                engine,
                requests,
                requests_impl,
                cli_api,
                info,
                transform_direction,
            )
            .await
        }
        TransformTypeInfo::Aggregate(info) => {
            create_aggregate_transform(
                engine,
                requests,
                requests_impl,
                cli_api,
                info,
                transform_direction,
            )
            .await
        }
    }
}

async fn create_map_filter_transform(
    engine: &WasmEngine,
    requests: DiffbeltRequests,
    requests_impl: DiffbeltRequestsClientImpl,
    cli_api: WasmCliApi,
    map_filter_wasm: &MapFilterTransformInfo,
    transform_direction: TransformDirection<'_>,
) -> Result<TransformEvaluator, RunTransformError> {
    let transform = MapFilterTransform::new(
        Box::from(transform_direction.from_collection_name),
        Box::from(transform_direction.to_collection_name),
        Box::from(transform_direction.reader_name),
    );

    let wasm_module_name = map_filter_wasm.wasm_module_name.as_str();
    let wasm_module = engine.get_module(wasm_module_name).await?;

    let wasm_instance = WasmModuleInstance::new(NewWasmInstanceOptions {
        engine,
        module: &wasm_module,
        requests: Arc::new(requests),
        cli_api: Some(cli_api),
    })
    .await?;

    let handler =
        MapFilterEvalHandler::new(wasm_instance, map_filter_wasm.wasm_method_name.as_str()).await?;

    Ok(TransformEvaluator {
        transform: TransformImpl::MapFilter(transform),
        eval_handler: FunctionEvalHandlerImpl::MapFilter(handler),
        requests_impl,
    })
}

async fn create_aggregate_transform(
    engine: &WasmEngine,
    requests: DiffbeltRequests,
    requests_impl: DiffbeltRequestsClientImpl,
    cli_api: WasmCliApi,
    aggregate: &AggregateTransformInfo,
    transform_direction: TransformDirection<'_>,
) -> Result<TransformEvaluator, RunTransformError> {
    let supports_accumulators_merge = aggregate.merge_accumulators.is_some();

    let transform = AggregateTransform::new(
        Box::from(transform_direction.from_collection_name),
        Box::from(transform_direction.to_collection_name),
        Box::from(transform_direction.reader_name),
        supports_accumulators_merge,
    );

    let wasm_module_name = aggregate.wasm_module_name.as_str();
    let wasm_module = engine.get_module(wasm_module_name).await?;

    let wasm_instance = WasmModuleInstance::new(NewWasmInstanceOptions {
        engine,
        module: &wasm_module,
        requests: Arc::new(requests),
        cli_api: Some(cli_api),
    })
    .await?;

    let handler = AggregateEvalHandler::new(wasm_instance, aggregate).await?;

    Ok(TransformEvaluator {
        transform: TransformImpl::Aggregate(transform),
        eval_handler: FunctionEvalHandlerImpl::Aggregate(handler),
        requests_impl,
    })
}
