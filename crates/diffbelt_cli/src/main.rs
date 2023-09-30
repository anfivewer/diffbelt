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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

type CommandResult = Result<(), CommandError>;

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

async fn run() {
    let args = Args::parse();

    let client = DiffbeltClient::new(DiffbeltClientNewOptions {
        host: "127.0.0.1".to_string(),
        port: 3030,
    });

    let state = Arc::new(CliState::new(client, args.config.clone()));

    let result = args.command.run(state.clone()).await;

    let exit_code = state.exit_code();

    let Err(err) = result else {
        exit(exit_code);
    };

    match err {
        CommandError::Message(msg) => {
            eprintln!("{msg}");
        }
        err => {
            eprintln!("Error: {:?}", err);
        }
    }

    exit(if exit_code == 0 { 1 } else { exit_code });
}

fn main() {
    let runtime = create_main_tokio_runtime().unwrap();

    runtime.block_on(run());
}
