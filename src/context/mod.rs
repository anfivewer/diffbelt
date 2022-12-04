use std::cell::Cell;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::raw_db::RawDb;
use crate::routes::Routing;

pub struct Context {
    pub routing: Routing,
    pub some_value: Arc<Mutex<Cell<i32>>>,
    pub raw_db: Arc<RawDb>,
}
