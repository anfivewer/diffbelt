use crate::collection::Collection;

use crate::database::config::DatabaseConfig;
pub use crate::database::database_inner::{DatabaseInner, GetReaderGenerationIdFnError};
use crate::raw_db::RawDb;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub mod config;
pub mod create_collection;
mod database_inner;
pub mod open;

pub struct Database {
    config: Arc<DatabaseConfig>,
    data_path: PathBuf,
    meta_raw_db: Arc<RawDb>,
    collections_alter_lock: Mutex<()>,
    collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
    inner: Arc<DatabaseInner>,
}
