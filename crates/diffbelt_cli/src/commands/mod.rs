use crate::commands::collection::Collection;
use crate::commands::collections::Collections;
use crate::commands::config_tests::Test;
use crate::state::CliState;
use crate::CommandResult;
use clap::Subcommand;
use std::sync::Arc;
use crate::commands::transform::Transform;

pub mod collection;
pub mod collections;
mod config_tests;
pub mod errors;
pub mod transform;

#[derive(Subcommand, Debug)]
pub enum Commands {
    Collections(Collections),
    Collection(Collection),
    Test(Test),
    Transform(Transform),
}

impl Commands {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        match self {
            Commands::Collections(collections) => collections.run(state).await,
            Commands::Collection(collection) => collection.run(state).await,
            Commands::Test(test) => test.run(state).await,
            Commands::Transform(transform) => transform.run(state).await,
        }
    }
}
