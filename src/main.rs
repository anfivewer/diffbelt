use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::get::CollectionGetOptions;
use crate::collection::methods::put::{CollectionPutManyOptions, CollectionPutOptions};
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::{KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId};
use crate::config::{Config, ReadConfigFromEnvError};
use crate::context::Context;
use crate::database::create_collection::CreateCollectionOptions;
use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;
use crate::raw_db::{RawDb, RawDbOptions};
use crate::routes::{BaseResponse, Response, StaticRouteOptions, StringResponse};
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;
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

    let path = Path::new(&config.data_path).join("_meta");
    let path = path.to_str().unwrap();

    let meta_raw_db = RawDb::open_raw_db(RawDbOptions {
        path,
        comparator: None,
        column_families: vec![],
    })
    .expect("Cannot open meta raw_db");

    let meta_raw_db = Arc::new(meta_raw_db);

    let database = Database::open(DatabaseOpenOptions {
        config: config.clone(),
        meta_raw_db: meta_raw_db.clone(),
    })
    .await
    .expect("Cannot open database");

    let collection = database
        .get_or_create_collection("test", CreateCollectionOptions { is_manual: false })
        .await
        .expect("Collection create");

    let manual_collection = database
        .get_or_create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .expect("Collection create");

    let result = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    println!("get result {:?}", result);

    let result = collection
        .put(CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
                value: Option::Some(OwnedCollectionValue::new(b"passed")),
                if_not_present: true,
            },
            generation_id: None,
            phantom_id: None,
        })
        .await;

    println!("put result {:?}", result);

    let result = collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate {
                    key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
                    value: Option::Some(OwnedCollectionValue::new(b"passed3")),
                    if_not_present: true,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey(b"test2".to_vec().into_boxed_slice()),
                    value: Option::Some(OwnedCollectionValue::new(b"passed again")),
                    if_not_present: true,
                },
            ],
            generation_id: None,
            phantom_id: None,
        })
        .await;

    println!("put result {:?}", result);

    let result = manual_collection
        .start_generation(StartGenerationOptions {
            generation_id: OwnedGenerationId(b"first".to_vec().into_boxed_slice()),
            abort_outdated: false,
        })
        .await;

    println!("start generation result {:?}", result);

    let result = manual_collection
        .put(CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
                value: Option::Some(OwnedCollectionValue::new(b"passed")),
                if_not_present: true,
            },
            generation_id: Some(OwnedGenerationId(b"first".to_vec().into_boxed_slice())),
            phantom_id: None,
        })
        .await;

    println!("put in manual result {:?}", result);

    let result = manual_collection
        .commit_generation(CommitGenerationOptions {
            generation_id: OwnedGenerationId(b"first".to_vec().into_boxed_slice()),
        })
        .await;

    println!("commit generation result {:?}", result);

    let result = manual_collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    println!("get from manual result {:?}", result);

    let context = Arc::new(RwLock::new(Context {
        config,
        routing: routes::new_routing(),
        meta_raw_db,
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

static mut TOKIO_RUNTIME: Option<Arc<Runtime>> = None;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let runtime = Arc::new(runtime);

    unsafe {
        TOKIO_RUNTIME = Some(runtime.clone());
    }

    runtime.block_on(run());
}
