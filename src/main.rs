use crate::config::Config;
use crate::context::Context;
use crate::raw_db::{RawDb, RawDbOptions};
use crate::routes::{BaseResponse, Response, StaticRouteOptions, StringResponse};
use std::cell::Cell;
use std::path::Path;
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
mod cursor;
mod database;
mod generation;
mod phantom;
mod raw_db;
mod reader;
mod routes;

#[derive(Debug)]
struct Error500;

impl Reject for Error500 {}

#[tokio::main]
async fn main() {
    let config = Config::read_from_env().expect("Config not parsed");

    let path = Path::new(&config.data_path).join("_meta");
    let path = path.to_str().unwrap();

    let database = RawDb::open_raw_db(RawDbOptions {
        path,
        comparator: None,
        column_families: vec![],
    })
    .expect("Cannot open meta raw_db");

    let context = Arc::new(RwLock::new(Context {
        config,
        routing: routes::new_routing(),
        raw_db: Arc::new(database),
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
