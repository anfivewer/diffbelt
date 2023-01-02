use std::sync::Arc;

use crate::config::{Config, ReadConfigFromEnvError};
use crate::context::Context;
use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;
use crate::http::routing;
use crate::http::server::start_http_server;
use crate::util::global_tokio_runtime::create_global_tokio_runtime;

mod collection;
mod common;
mod config;
mod context;
mod database;
mod http;
mod protos;
mod raw_db;
#[cfg(test)]
mod tests;
mod util;

async fn run() {
    let config = Config::read_from_env();
    let config = match config {
        Ok(config) => config,
        Err(err) => match err {
            ReadConfigFromEnvError::VarNotPresent(name) => {
                eprintln!("ENV variable \"{}\" not specified", name);
                std::process::exit(1);
            }
            rest => panic!("Config reading error: {:?}", rest),
        },
    };
    let config = Arc::new(config);

    let database = Database::open(DatabaseOpenOptions {
        data_path: &config.data_path,
        config: Arc::new(Default::default()),
    })
    .await
    .expect("Cannot open database");

    let mut context = Context {
        config,
        routing: routing::Routing::new(),
        database: Arc::new(database),
    };

    routing::register_routes::register_routes(&mut context);

    let context = Arc::new(context);

    start_http_server(context).await;
}

fn main() {
    let runtime = create_global_tokio_runtime().unwrap();

    runtime.block_on(run());
}
