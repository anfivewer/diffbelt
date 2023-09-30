use crate::commands::errors::CommandError;
use crate::state::CliState;
use crate::CommandResult;
use clap::Parser;
use diffbelt_cli_config::config_tests::run::run_tests;
use std::str::from_utf8;
use std::sync::Arc;

#[derive(Parser, Debug)]
pub struct Test;

impl Test {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        let config_path = state
            .config_path
            .as_ref()
            .map(|x| x.as_str())
            .ok_or_else(|| {
                CommandError::Message("Specify config path with --config parameter\n\nExample: diffbelt_cli --config config.yaml test".to_string())
            })?;

        let bytes = tokio::fs::read(config_path)
            .await
            .map_err(CommandError::Io)?;
        let bytes = bytes.as_slice();
        let content = from_utf8(bytes).map_err(CommandError::Utf8)?;

        let is_ok = run_tests(content).map_err(CommandError::RunTests)?;

        if !is_ok {
            state.set_non_zero_exit_code(1);
        }

        Ok(())
    }
}
