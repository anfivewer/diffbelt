use crate::collection::util::record_key::RecordKey;
use crate::common::{CollectionKey, GenerationId, OwnedCollectionKey, PhantomId};
use crate::raw_db::query::{
    QueryDirection, QueryDirectionBackward, QueryDirectionForward, QueryKeysOnly, QueryOptions,
    QueryState,
};
use crate::raw_db::{RawDb, RawDbError};
use rocksdb::DB;

pub struct RawDbGetKeysAroundOptions<'a> {
    pub record_key: RecordKey<'a>,
    pub limit: usize,
    pub records_to_view_limit: usize,
}

pub struct RawDbGetKeysAroundResult {
    pub left: Vec<OwnedCollectionKey>,
    pub right: Vec<OwnedCollectionKey>,
    pub has_more_on_the_left: bool,
    pub has_more_on_the_right: bool,
}

impl RawDb {
    pub fn keys_around_sync(
        &self,
        options: RawDbGetKeysAroundOptions<'_>,
    ) -> Result<RawDbGetKeysAroundResult, RawDbError> {
        let record_key = options.record_key;
        let generation_id = record_key.get_generation_id();
        let phantom_id = record_key.get_phantom_id();

        let limit = options.limit;
        let records_to_view_limit = options.records_to_view_limit;

        let db = self.db.get_db();

        let mut result = RawDbGetKeysAroundResult {
            left: Vec::with_capacity(limit),
            right: Vec::with_capacity(limit),
            has_more_on_the_left: false,
            has_more_on_the_right: false,
        };

        let start_key = record_key.get_collection_key();
        let phantom_id = phantom_id.to_opt_if_empty();

        process_direction(
            QueryDirectionForward,
            db,
            start_key,
            generation_id,
            phantom_id,
            limit,
            records_to_view_limit,
            &mut result.has_more_on_the_right,
            &mut result.right,
        )?;

        process_direction(
            QueryDirectionBackward,
            db,
            start_key,
            generation_id,
            phantom_id,
            limit,
            records_to_view_limit,
            &mut result.has_more_on_the_left,
            &mut result.left,
        )?;

        Ok(result)
    }
}

fn process_direction<D: QueryDirection>(
    direction: D,
    db: &DB,
    start_key: CollectionKey<'_>,
    generation_id: GenerationId<'_>,
    phantom_id: Option<PhantomId<'_>>,
    limit: usize,
    records_to_view_limit: usize,
    has_more: &mut bool,
    result: &mut Vec<OwnedCollectionKey>,
) -> Result<(), RawDbError> {
    let mut query = QueryState::new(
        db,
        QueryOptions {
            kind: QueryKeysOnly,
            direction,
            start_key: Some(start_key),
            generation_id,
            phantom_id,
            continuation_state: None,
            records_to_view_limit,
        },
    )?;

    let mut count = 0;

    {
        let item = query.next();
        let Some(item) = item else {
            return Err(RawDbError::CursorDidNotFoundRecord);
        };

        let key = item?;
        let collection_key = key.get_collection_key();

        if collection_key != start_key {
            return Err(RawDbError::CursorDidNotFoundRecord);
        }
    }

    for item in query.by_ref() {
        if count >= limit {
            *has_more = true;
            break;
        }

        let item = item?;

        result.push(item.get_collection_key().to_owned());

        count += 1;
    }

    Ok(())
}
