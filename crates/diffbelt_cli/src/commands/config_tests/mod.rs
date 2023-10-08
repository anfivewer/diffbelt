use crate::commands::errors::CommandError;
use crate::state::CliState;
use crate::CommandResult;
use clap::Parser;
use diffbelt_cli_config::config_tests::run::run_tests;
use std::sync::Arc;

#[derive(Parser, Debug)]
pub struct Test;

impl Test {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        let config = state.require_config()?;

        let is_ok = run_tests(config).map_err(CommandError::RunTests)?;

        if !is_ok {
            state.set_non_zero_exit_code(1);
        }

        Ok(())
    }
}
