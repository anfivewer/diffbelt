use crate::collection::cursor::query::QueryCursorPack;

use crate::collection::methods::errors::CollectionMethodError;
use crate::database::config::DatabaseConfig;
use crate::database::cursors::query::QueryCursor;
use crate::raw_db::query_collection_records::{
    QueryCollectionRecordsOptions, QueryCollectionRecordsResult,
};
use crate::raw_db::RawDb;
use std::sync::Arc;

pub struct GetPackOptions {
    pub db: Arc<RawDb>,
    pub config: Arc<DatabaseConfig>,
}

impl QueryCursor {
    pub fn get_pack_sync(
        &self,
        options: GetPackOptions,
    ) -> Result<QueryCursorPack, CollectionMethodError> {
        let GetPackOptions { db, config } = options;

        let phantom_id = self.phantom_id.as_ref().map(|id| id.as_ref());

        let (last_record_key, from_record_key) = match &self.last_and_next_record_key {
            Some(last_and_next) => (
                Some(last_and_next.last.as_ref()),
                Some(last_and_next.next.as_ref()),
            ),
            None => (None, None),
        };

        let result = db.query_collection_records_sync(QueryCollectionRecordsOptions {
            generation_id: self.generation_id.as_ref(),
            phantom_id: phantom_id.clone(),
            last_record_key,
            from_record_key,
            limit: config.query_pack_limit,
            records_to_view_limit: config.query_pack_records_limit,
        })?;

        let QueryCollectionRecordsResult {
            items,
            last_and_next_record_key,
        } = result;

        Ok(QueryCursorPack {
            items,
            last_and_next_record_key,
        })
    }
}
