mod run;
mod run_parse;

use crate::commands::transform::run::{run_transform_command, Run};
use crate::state::CliState;
use crate::CommandResult;
use clap::{Parser, Subcommand};
use std::sync::Arc;

#[derive(Parser, Debug)]
pub struct Transform {
    #[command(subcommand)]
    command: TransformSubcommand,
}

#[derive(Subcommand, Debug)]
enum TransformSubcommand {
    /// Run transform by name
    Run(Run),
}

impl Transform {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        let Transform { command } = self;
        match command {
            TransformSubcommand::Run(run) => {
                let Run { run } = run;

                run_transform_command(run, state).await
            }
        }
    }
}
