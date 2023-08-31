mod commands;
pub mod format;
mod state;

use crate::commands::errors::CommandError;
use crate::commands::Commands;
use crate::state::CliState;
use clap::Parser;
use diffbelt_http_client::client::{DiffbeltClient, DiffbeltClientNewOptions};
use diffbelt_util::tokio_runtime::create_main_tokio_runtime;
use std::process::exit;
use std::sync::Arc;

type CommandResult = Result<(), CommandError>;

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

async fn run() {
    let args = Args::parse();

    let client = DiffbeltClient::new(DiffbeltClientNewOptions {
        host: "127.0.0.1".to_string(),
        port: 3030,
    });

    let state = Arc::new(CliState { client });

    let result = args.command.run(state).await;

    let Err(err) = result else {
        return;
    };

    println!("Error: {:?}", err);

    exit(1);
}

fn main() {
    let runtime = create_main_tokio_runtime().unwrap();

    runtime.block_on(run());
}
