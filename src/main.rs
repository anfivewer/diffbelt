use crate::context::Context;
use std::cell::Cell;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::response::Builder;
use warp::http::StatusCode;
use warp::path::FullPath;
use warp::reject::Reject;
use warp::{Filter, Rejection};
use crate::routes::{StaticRouteOptions, Response, StringResponse, BaseResponse};

mod common;
mod context;
mod raw_db;
mod database;
mod collection;
mod generation;
mod phantom;
mod reader;
mod cursor;
mod routes;

#[derive(Debug)]
struct Error500;

impl Reject for Error500 {}

#[tokio::main]
async fn main() {
    let database = raw_db::create_raw_db();

    let context = Arc::new(Mutex::new(Context {
        routing: routes::new_routing(),
        some_value: Arc::new(Mutex::new(Cell::new(0))),
        raw_db: Arc::new(database),
    }));

    async {
        let mut context = context.lock().await;
        routes::root::register_root_route(&mut context);
    }
    .await;

    let routed_get = warp::get()
        .map(move || context.clone())
        .and(warp::path::full())
        .and_then(|context: Arc<Mutex<Context>>, path: FullPath| async move {
            let path = path.as_str();

            let locked_context = context.lock().await;

            let static_route = locked_context.routing.static_get_routes.get(path);

            let static_route = match static_route {
                None => {
                    return Err(warp::reject::not_found());
                }
                Some(static_route) => static_route.clone(),
            };

            drop(locked_context);

            let result = static_route(StaticRouteOptions { context: &context }).await;

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
