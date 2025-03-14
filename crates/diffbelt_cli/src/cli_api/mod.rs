use crate::commands::transform::runner::transform_config::TransformConfig;
use crate::commands::transform::runner::TransformRunner;
use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::wasm::cli_api::WasmCliApi;
use diffbelt_cli_config::wasm::engine::{WasmEngine, WasmEngineOptions};
use diffbelt_cli_config::CliConfig;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_util::tokio::{spawn_async_thread_local, spawn_blocking_async};
use diffbelt_util_no_std::on_drop::OnDrop;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, oneshot};

pub struct CliApiStopToken {
    stop_sender: Option<oneshot::Sender<()>>,
}

impl Drop for CliApiStopToken {
    fn drop(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            sender.send(()).unwrap_or(());
        }
    }
}

pub async fn create_cli_api(
    config: &CliConfig,
    wasm_engine: WasmEngine,
    client: Arc<DiffbeltClient>,
) -> Result<(WasmCliApi, CliApiStopToken), RunTransformError> {
    let (sender, mut receiver) = mpsc::channel(64);

    let wasm_api = WasmCliApi::new(sender);

    let config = TransformConfig::new(config)?;

    let runner = TransformRunner::new(config, wasm_engine, client, wasm_api.clone());

    let (stop_sender, mut stop_receiver) = oneshot::channel();

    spawn_async_thread_local(|| async move {
        loop {
            let task = select! {
                task = receiver.recv() => {
                    task
                },
                _ = &mut stop_receiver => {
                    break;
                },
            };

            let Some(task) = task else {
                break;
            };

            let (collection_name, sender) = task;

            let result = runner.run_transform(&collection_name).await;

            let () = sender.send(result).unwrap_or(());
        }
    })
    .await;

    Ok((
        wasm_api,
        CliApiStopToken {
            stop_sender: Some(stop_sender),
        },
    ))
}
