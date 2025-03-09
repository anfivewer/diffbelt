use std::sync::Arc;

use clap::Parser;

use crate::commands::errors::CommandError;
use crate::state::CliState;
use crate::CommandResult;
use diffbelt_cli_config::config_tests::run::run_tests;
use diffbelt_cli_config::config_tests::RunTestsOptions;

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

        let mut options = RunTestsOptions {
            with_unit: false,
            with_integration: false,
            client: Some(state.client.clone()),
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

        Ok(())
    }
}
