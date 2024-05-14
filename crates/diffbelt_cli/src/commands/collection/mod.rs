use std::sync::Arc;

use clap::Parser;

use crate::commands::collection::get::get_collection_command;
use crate::state::CliState;
use crate::CommandResult;

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
