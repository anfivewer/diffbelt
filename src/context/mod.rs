use crate::config::Config;
use crate::raw_db::RawDb;
use crate::routes::Routing;
use std::cell::Cell;
use std::sync::Arc;

pub struct Context {
    pub config: Config,
    pub routing: Routing,
    pub raw_db: Arc<RawDb>,
}
