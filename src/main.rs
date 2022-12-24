use crate::config::{Config, ReadConfigFromEnvError};
use crate::context::Context;

use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;

use crate::routes::{BaseResponse, Response, StaticRouteOptions, StringResponse};
use crate::util::global_tokio_runtime::create_global_tokio_runtime;

use std::sync::Arc;

use tokio::sync::RwLock;
use warp::http::response::Builder;
use warp::http::StatusCode;
use warp::path::FullPath;
use warp::reject::Reject;
use warp::{Filter, Rejection};

mod collection;
mod common;
mod config;
mod context;
mod database;
mod protos;
mod raw_db;
mod routes;
#[cfg(test)]
mod tests;
mod util;

#[derive(Debug)]
struct Error500;

impl Reject for Error500 {}

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

    let context = Arc::new(RwLock::new(Context {
        config,
        routing: routes::new_routing(),
        database: Arc::new(database),
    }));

    async {
        let mut context = context.write().await;
        routes::root::register_root_route(&mut context);
    }
    .await;

    let routed_get = warp::get()
        .map(move || context.clone())
        .and(warp::path::full())
        .and_then(|context: Arc<RwLock<Context>>, path: FullPath| async move {
            let path = path.as_str();

            let locked_context = context.read().await;

            let static_route = locked_context.routing.static_get_routes.get(path);

            let static_route = match static_route {
                None => {
                    return Err(warp::reject::not_found());
                }
                Some(static_route) => static_route.clone(),
            };

            drop(locked_context);

            let result = static_route(StaticRouteOptions { context }).await;

            let response_builder = Builder::new();

            match result {
                Response::String(StringResponse {
                    base: BaseResponse { status },
                    str,
                }) => response_builder
                    .status(status)
                    .body(str)
                    .map_err(|_| warp::reject::custom(Error500)),
            }
        })
        .recover(|err: Rejection| async move {
            if let Some(_) = err.find::<Error500>() {
                return Ok(warp::reply::with_status(
                    "500",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }

            Err(err)
        });

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    let hello2 = warp::path!("hello2" / String).map(|name| format!("Hello2, {}!", name));

    let routes = routed_get.or(hello).or(hello2);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

fn main() {
    let runtime = create_global_tokio_runtime().unwrap();

    runtime.block_on(run());
}
