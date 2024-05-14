use std::sync::Arc;

use crate::collection::Collection;
use crate::database::Database;

impl Database {
    pub async fn collections_list(&self) -> Vec<Arc<Collection>> {
        let collections = self.collections.read().await;
        let result: Vec<Arc<Collection>> = collections
            .values()
            .map(|collection| collection.clone())
            .collect();

        result
    }
}
