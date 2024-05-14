use std::sync::Arc;

use clap::{Parser, Subcommand};

use crate::state::CliState;
use crate::CommandResult;

#[derive(Parser, Debug)]
pub struct Collections {
    #[command(subcommand)]
    command: Option<CollectionsSubcommand>,
}

#[derive(Subcommand, Debug)]
enum CollectionsSubcommand {
    /// Lists collections
    List,
    /// Alias to list
    Ls,
}

impl Collections {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        let response = state.client.list_collections().await.unwrap();

        for item in response.items {
            println!(
                "{} {}",
                item.name,
                if item.is_manual {
                    "manual"
                } else {
                    "non-manual"
                }
            );
        }

        Ok(())
    }
}
