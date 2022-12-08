use crate::config::Config;
use crate::raw_db::RawDb;
use crate::routes::Routing;
use std::sync::Arc;

pub struct Context {
    pub config: Arc<Config>,
    pub routing: Routing,
    pub meta_raw_db: Arc<RawDb>,
}
