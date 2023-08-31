use crate::commands::collection::get::get_collection_command;
use crate::state::CliState;
use crate::CommandResult;
use clap::Parser;
use std::sync::Arc;

pub mod get;

#[derive(Parser, Debug)]
pub struct Collection {
    #[command()]
    name: String,
}

impl Collection {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        get_collection_command(self, state).await
    }
}
