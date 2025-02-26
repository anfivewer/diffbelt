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
use diffbelt_protos::protos::api::collection::{
    CreateCollectionRequestArgs, CreateCollectionResponse,
};
use diffbelt_protos::protos::api::common::ErrorResponse;
use diffbelt_protos::protos::api::methods::RequestArgs;
use diffbelt_protos::protos::handlers::{ApiHandler, CreateCollectionApiHandler};
use diffbelt_protos::protos::impls::RequestProto;
use diffbelt_protos::Serializer;

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
                for collection in &config.collections {
                    let existing = existing_collections_is_manual.get(collection.name.as_ref());
                    if let Some(is_manual) = existing {
                        if collection.manual == *is_manual {
                            continue;
                        } else if *is_manual {
                            return Err(CommandError::Message(format!(
                                "Collection {} already exists and is manual, but should not",
                                collection.name
                            )));
                        } else {
                            return Err(CommandError::Message(format!(
                                "Collection {} already exists and is not manual, but should be",
                                collection.name
                            )));
                        }
                    }

                    println!(
                        "Create {}collection {}...",
                        if collection.manual { "manual " } else { "" },
                        &collection.name
                    );

                    let mut serializer = Serializer::<RequestProto>::new();
                    let collection_name = Some(serializer.create_string(collection.name.as_ref()));
                    let request = CreateCollectionApiHandler::create_request(
                        serializer,
                        CreateCollectionRequestArgs {
                            collection_name,
                            is_manual: collection.manual,
                        },
                    );
                    let response = state
                        .client
                        .flatbuffers_call::<CreateCollectionApiHandler>(request)
                        .await?;

                    match response.response() {
                        Ok(_) => (),
                        Err(Some(err)) => {
                            return Err(CommandError::Message(format!(
                                "Error code {}, reason: {}, details: {}",
                                err.code(),
                                err.reason().unwrap_or("()"),
                                err.details().unwrap_or("()"),
                            )));
                        }
                        Err(None) => {
                            return Err(CommandError::Message(String::from("No response")));
                        }
                    };

                    execute!(
                        io::stdout(),
                        MoveToPreviousLine(1),
                        Clear(ClearType::CurrentLine),
                        MoveToColumn(0)
                    )?;
                    println!(
                        "{} {} created",
                        if collection.manual {
                            "Manual collection"
                        } else {
                            "Collection"
                        },
                        &collection.name,
                    );
                }
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
