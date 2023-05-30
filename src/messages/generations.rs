use crate::database::DatabaseInner;
use std::sync::Arc;

pub enum DatabaseCollectionGenerationsTask {
    Init(Arc<DatabaseInner>),
}
