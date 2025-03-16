use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::sync::Arc;

use crate::commands::errors::CommandError;
use crate::state::CliState;
use crate::CommandResult;
use clap::{Parser, Subcommand};
use crossterm::cursor::{MoveToColumn, MoveToPreviousLine};
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use diffbelt_cli_config::util::init_collections::{
    init_collections, InitCollectionsError, InitCollectionsOptions,
};

#[derive(Parser, Debug)]
pub struct Collections {
    #[command(subcommand)]
    command: CollectionsSubcommand,
}

#[derive(Subcommand, Debug)]
enum CollectionsSubcommand {
    /**
       Creates all collections from config, returns error if some existing collection type (manual
       or auto) is not matched
    */
    Init,
    /// Lists collections
    List,
    /// Alias to list
    Ls,
}

impl Collections {
    pub async fn run(&self, state: Arc<CliState>) -> CommandResult {
        let response = state.client.list_collections().await?;

        let mut existing_collections_is_manual = HashMap::with_capacity(response.items.len());
        for item in &response.items {
            existing_collections_is_manual.insert(item.name.as_str(), item.is_manual);
        }

        match &self.command {
            CollectionsSubcommand::Init => {
                let config = state.require_config()?;
                init_collections(InitCollectionsOptions {
                    client: &state.client,
                    collections: &config.collections,
                    transforms: &config.transforms,
                    print_before_create: |collection| {
                        println!(
                            "Create {}collection {}...",
                            if collection.manual { "manual " } else { "" },
                            &collection.name
                        );
                    },
                    print_after_create: |collection| {
                        execute!(
                            io::stdout(),
                            MoveToPreviousLine(1),
                            Clear(ClearType::CurrentLine),
                            MoveToColumn(0)
                        )
                        .expect("IO error");
                        println!(
                            "{} {} created",
                            if collection.manual {
                                "Manual collection"
                            } else {
                                "Collection"
                            },
                            &collection.name,
                        );
                    },
                })
                .await
                .map_err(|err| match err {
                    InitCollectionsError::Message(msg) => CommandError::Message(msg),
                    InitCollectionsError::DiffbeltClient(err) => err.into(),
                })?;
            }
            CollectionsSubcommand::List | CollectionsSubcommand::Ls => {
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
            }
        }

        Ok(())
    }
}
