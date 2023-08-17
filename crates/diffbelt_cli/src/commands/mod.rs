use crate::commands::collection::Collection;
use crate::commands::collections::Collections;
use crate::state::CliState;
use crate::CommandResult;
use clap::Subcommand;
use std::sync::Arc;

pub mod collection;
pub mod collections;
pub mod errors;

#[derive(Subcommand, Debug)]
pub enum Commands {
    Collections(Collections),
    Collection(Collection),
}

impl Commands {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        match self {
            Commands::Collections(collections) => collections.run(state).await,
            Commands::Collection(collection) => collection.run(state).await,
        }
    }
}
