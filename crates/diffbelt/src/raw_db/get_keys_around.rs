use crate::collection::constants::{COLLECTION_CF_META, COLLECTION_META_GC_PHANTOM_ID_KEY};
use rocksdb::{DBPinnableSlice, WriteBatch, DB};

use crate::collection::util::record_key::RecordKey;
use crate::common::{CollectionKey, GenerationId, IsByteArray, OwnedCollectionKey, PhantomId};
use crate::raw_db::query::{
    QueryDirection, QueryDirectionBackward, QueryDirectionForward, QueryKeysOnly, QueryOptions,
    QueryState,
};
use crate::raw_db::{RawDb, RawDbError};

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

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let is_phantom_exists = phantom_id.is_none()
            || phantom_id
                .map(|x| db.get_pinned_cf(&meta_cf, &Self::prefixed_phantom_id_key(x)))
                .transpose()?
                .flatten()
                .is_some();

        if !is_phantom_exists {
            return Err(RawDbError::NoSuchPhantom);
        }

        let gc_phantom_id_a = db.get_pinned_cf(&meta_cf, COLLECTION_META_GC_PHANTOM_ID_KEY)?;
        let gc_phantom_id_b = db.get_pinned_cf(&meta_cf, COLLECTION_META_GC_PHANTOM_ID_KEY)?;

        let mut result = RawDbGetKeysAroundResult {
            left: Vec::with_capacity(limit),
            right: Vec::with_capacity(limit),
            has_more_on_the_left: false,
            has_more_on_the_right: false,
        };

        let start_key = record_key.get_collection_key();

        process_direction(
            QueryDirectionForward,
            db,
            start_key,
            generation_id,
            phantom_id,
            gc_phantom_id_a,
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
            gc_phantom_id_b,
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
    gc_phantom_id: Option<DBPinnableSlice<'_>>,
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
            gc_phantom_id,
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

    if !query.records_to_delete.is_empty() {
        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let mut batch = WriteBatch::default();

        for record_key in query.records_to_delete.drain(..) {
            batch.delete_cf(&meta_cf, record_key.get_byte_array());
        }

        db.write(batch)?;
    }

    Ok(())
}
