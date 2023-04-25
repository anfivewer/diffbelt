use crate::collection::Collection;

use crate::database::config::DatabaseConfig;
pub use crate::database::database_inner::{DatabaseInner, GetReaderGenerationIdFnError};
use crate::raw_db::RawDb;
use crate::util::atomic_cleanup::AtomicCleanup;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{watch, Mutex, RwLock};

pub mod config;
pub mod constants;
pub mod create_collection;
pub mod cursors;
mod database_inner;
mod drop;
pub mod list_collections;
pub mod open;
mod readers;

pub struct Database {
    config: Arc<DatabaseConfig>,
    data_path: PathBuf,
    database_raw_db: Arc<RawDb>,
    collections_alter_lock: Mutex<()>,
    collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
    inner: Arc<DatabaseInner>,
    stop_sender: AtomicCleanup<watch::Sender<bool>>,
}
