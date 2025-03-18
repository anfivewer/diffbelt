use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

use crate::cli_api::create_cli_api;
use crate::commands::errors::CommandError;
use crate::state::CliState;
use crate::CommandResult;
use diffbelt_cli_config::config_tests::run::run_tests;
use diffbelt_cli_config::config_tests::RunTestsOptions;
use diffbelt_cli_config::wasm::engine::{WasmEngine, WasmEngineOptions};

#[derive(Parser, Debug)]
pub struct Test {
    #[arg(long)]
    unit: bool,
    #[arg(long)]
    integration: bool,
}

impl Test {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        let config = state.require_config()?;

        let wasm_engine = WasmEngine::new(WasmEngineOptions {
            wasm_root_path: PathBuf::from(config.self_path.as_ref()),
        })
        .await?;

        let (cli_api, stop_token) =
            create_cli_api(config, wasm_engine.clone(), state.client.clone()).await?;

        let mut options = RunTestsOptions {
            with_unit: false,
            with_integration: false,
            wasm_engine,
            client: Some(state.client.clone()),
            cli_api: Some(cli_api),
        };

        let mut with_filter = false;

        if self.unit {
            with_filter = true;
            options.with_unit = true;
        }
        if self.integration {
            with_filter = true;
            options.with_integration = true;
        }
        if !with_filter {
            options.with_unit = true;
            options.with_integration = true;
        }

        let is_ok = run_tests(config, options)
            .await
            .map_err(CommandError::RunTests)?;

        if !is_ok {
            state.set_non_zero_exit_code(1);
        }

        drop(stop_token);

        Ok(())
    }
}
