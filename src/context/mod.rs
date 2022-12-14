use crate::config::Config;
use crate::database::Database;

use crate::routes::Routing;
use std::sync::Arc;

pub struct Context {
    pub config: Arc<Config>,
    pub routing: Routing,
    pub database: Arc<Database>,
}
