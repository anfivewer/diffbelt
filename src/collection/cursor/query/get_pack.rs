use crate::collection::cursor::query::{QueryCursor, QueryCursorPack};

use crate::collection::methods::errors::CollectionMethodError;
use crate::database::config::DatabaseConfig;
use crate::raw_db::query_collection_records::{
    QueryCollectionRecordsOptions, QueryCollectionRecordsResult,
};
use crate::raw_db::RawDb;
use std::sync::Arc;

pub struct GetPackOptions {
    pub this_cursor_id: Option<String>,
    pub db: Arc<RawDb>,
    pub config: Arc<DatabaseConfig>,
}

impl QueryCursor {
    pub fn get_pack_sync(
        &self,
        options: GetPackOptions,
    ) -> Result<QueryCursorPack, CollectionMethodError> {
        let GetPackOptions {
            this_cursor_id,
            db,
            config,
        } = options;

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

        let next_cursor = match last_and_next_record_key {
            None => None,
            last_and_next_record_key => Some(QueryCursor {
                prev_cursor_id: this_cursor_id,
                generation_id: self.generation_id.clone(),
                phantom_id: self.phantom_id.clone(),
                last_and_next_record_key,
            }),
        };

        Ok(QueryCursorPack { items, next_cursor })
    }
}
