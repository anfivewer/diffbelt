use std::sync::Arc;
use diffbelt_util::idling_status::IdlingStatus;

use crate::config::Config;
use crate::database::Database;
use crate::http::routing::Routing;

pub struct Context {
    pub config: Arc<Config>,
    pub routing: Routing,
    pub database: Arc<Database>,
    pub idling: IdlingStatus,
}
