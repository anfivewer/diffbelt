use diffbelt_util::idling_status::IdlingStatus;
use std::sync::Arc;
use std::time::Duration;

use crate::config::{Config, ReadConfigFromEnvError};
use crate::context::Context;
use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;
use crate::http::routing;
use crate::http::server::start_http_server;
use crate::util::tokio_runtime::create_main_tokio_runtime;

mod collection;
mod common;
mod config;
mod context;
mod database;
mod http;
pub mod messages;
mod protos;
mod raw_db;
#[cfg(test)]
mod tests;
mod util;

async fn run() {
    let idling = IdlingStatus::new();
    let task = idling.start_work();

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

    if config.is_clear {
        std::fs::remove_dir_all(&config.data_path).expect("cannot remove data_path");
    }

    let database = Database::open(DatabaseOpenOptions {
        data_path: &config.data_path,
        config: Arc::new(Default::default()),
        idling: idling.clone(),
    })
    .await
    .expect("Cannot open database");

    let mut context = Context {
        config,
        routing: routing::Routing::new(),
        database: Arc::new(database),
        idling: idling.clone(),
    };

    routing::register_routes::register_routes(&mut context);

    let context = Arc::new(context);

    tokio::spawn(async move {
        start_http_server(context, task).await;
    });

    loop {
        () = idling.on_idle_for(Duration::from_millis(500)).await;

        println!("IDLE");

        () = idling.on_busy().await;

        println!("BUSY");
    }
}

fn main() {
    let runtime = create_main_tokio_runtime().unwrap();

    runtime.block_on(run());
}
