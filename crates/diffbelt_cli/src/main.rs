mod commands;
pub mod format;
mod global;
mod state;

use crate::commands::errors::CommandError;
use crate::commands::Commands;
use crate::global::set_global_config;
use crate::state::CliState;
use clap::{Arg, Command, Parser};
use diffbelt_cli_config::CliConfig;
use diffbelt_http_client::client::{DiffbeltClient, DiffbeltClientNewOptions};
use diffbelt_util::tokio_runtime::create_main_tokio_runtime;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::rc::Rc;
use std::str::from_utf8;

use std::sync::Arc;

type CommandResult = Result<(), CommandError>;

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[arg(long)]
    verbose: bool,
    #[arg(short, long)]
    config: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

async fn run() {
    let pre_cli = Command::new("CLI")
        .arg(Arg::new("verbose").long("verbose"))
        .arg(Arg::new("config").short('c').long("config"))
        .ignore_errors(true);

    let pre_cli_matches = pre_cli.try_get_matches().map(Some).unwrap_or(None);

    let config = 'outer: {
        match pre_cli_matches {
            Some(matches) => {
                let config_path = matches.get_one::<String>("config");
                let Some(config_path) = config_path.map(|x| x.as_str()) else {
                    break 'outer None;
                };

                let config_path = PathBuf::from(config_path);

                let Some(self_path) = config_path.parent() else {
                    eprintln!(
                        "Config path has no parent: {}",
                        config_path.as_os_str().to_string_lossy()
                    );
                    exit(1);
                };
                let Some(self_path) = self_path.to_str() else {
                    eprintln!("Cannot transform self_path to str");
                    exit(1);
                };
                let self_path = Rc::from(self_path);

                let bytes = match tokio::fs::read(config_path).await {
                    Ok(x) => x,
                    Err(err) => {
                        eprintln!("Error when reading config: {}", err.to_string());
                        exit(1);
                    }
                };
                let bytes = bytes.as_slice();
                let content = match from_utf8(bytes) {
                    Ok(x) => x,
                    Err(err) => {
                        eprintln!("Error when reading config: {}", err.to_string());
                        exit(1);
                    }
                };

                let config = match CliConfig::from_str(self_path, content) {
                    Ok(x) => x,
                    Err(err) => {
                        eprintln!("Error when parsing config: {err:?}");
                        exit(1);
                    }
                };

                Some(Rc::new(config))
            }
            None => None,
        }
    };

    if let Some(config) = config.as_ref() {
        set_global_config(config.clone());
    }

    let args = Args::parse();

    let client = DiffbeltClient::new(DiffbeltClientNewOptions {
        host: "127.0.0.1".to_string(),
        port: 3030,
    });

    let state = Arc::new(CliState::new(client, config, args.verbose));

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
