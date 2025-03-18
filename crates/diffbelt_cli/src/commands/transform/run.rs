use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;

use crate::cli_api::create_cli_api;
use crate::commands::errors::CommandError;
use crate::commands::transform::runner::TransformRunner;
use crate::state::CliState;
use crate::CommandResult;
use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::wasm::engine::{WasmEngine, WasmEngineOptions};

#[derive(Args, Debug)]
pub struct Run {
    #[command(subcommand)]
    pub run: RunSubcommand,
}

#[derive(Clone, Debug)]
pub struct RunSubcommand {
    pub name: String,
}

pub async fn run_transform_command(command: &RunSubcommand, state: Arc<CliState>) -> CommandResult {
    let client = state.client.clone();
    let config = state.require_config()?;

    let wasm_engine = WasmEngine::new(WasmEngineOptions {
        wasm_root_path: PathBuf::from(config.self_path.as_ref()),
    })
    .await?;

    let (cli_api, stop_token) = create_cli_api(config, wasm_engine, client.clone()).await?;

    let RunSubcommand { name } = command;

    let () = cli_api.run_transform(name.clone()).await?;

    drop(stop_token);

    Ok(())
}
