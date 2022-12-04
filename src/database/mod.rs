use crate::collection::Collection;
use crate::raw_db::RawDb;
use std::collections::HashMap;
use std::sync::Arc;

struct Database {
    raw_db: Arc<RawDb>,
    collections: HashMap<String, Collection>,
}
