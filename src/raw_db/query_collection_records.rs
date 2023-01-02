use crate::collection::util::record_key::{
    OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKey, RecordKey,
};
use crate::common::{
    CollectionKey, GenerationId, IsByteArray, KeyValue, OwnedCollectionValue, PhantomId,
};
use crate::raw_db::cursor_util::{get_initial_last_record, LastRecord};
use crate::raw_db::{RawDb, RawDbError};
use rocksdb::{Direction, IteratorMode};

pub struct QueryCollectionRecordsOptions<'a> {
    pub generation_id: GenerationId<'a>,
    pub phantom_id: Option<PhantomId<'a>>,
    // Specified if query has lower bound
    // if `last_record_key` is specified, this MUST be too
    // TODO: receive `LastAndNextRecordKey` or this field by enum to lower chance of misuse
    pub from_record_key: Option<RecordKey<'a>>,
    // Passed if this is continuation of previous query
    pub last_record_key: Option<RecordKey<'a>>,
    pub limit: usize,
    pub records_to_view_limit: usize,
}

pub struct LastAndNextRecordKey {
    pub last: OwnedRecordKey,
    pub next: OwnedRecordKey,
}

pub struct QueryCollectionRecordsResult {
    pub items: Vec<KeyValue>,
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

impl RawDb {
    pub fn query_collection_records_sync(
        &self,
        options: QueryCollectionRecordsOptions<'_>,
    ) -> Result<QueryCollectionRecordsResult, RawDbError> {
        let QueryCollectionRecordsOptions {
            generation_id,
            phantom_id,
            from_record_key,
            last_record_key,
            limit,
            records_to_view_limit,
        } = options;

        let iterator_mode = match from_record_key.as_ref() {
            Some(record_key) => IteratorMode::From(record_key.get_byte_array(), Direction::Forward),
            None => IteratorMode::Start,
        };

        let db = self.db.get_db();

        let mut iterator = db.iterator(iterator_mode);

        let last_record = get_initial_last_record(
            db,
            &mut iterator,
            last_record_key,
            from_record_key.is_some(),
        )?;
        let mut last_record = match last_record {
            Some(x) => x,
            None => {
                return Ok(QueryCollectionRecordsResult {
                    items: vec![],
                    last_and_next_record_key: None,
                });
            }
        };

        let mut next_record_key = None;
        let mut result = Vec::with_capacity(limit);
        let mut count = 0;
        let mut records_seen = 0;

        for kv in iterator {
            let kv: (Box<[u8]>, Box<[u8]>) = kv?;
            let (key, value) = kv;

            let record_key = OwnedParsedRecordKey::from_boxed_slice(key)
                .or(Err(RawDbError::InvalidRecordKey))?;

            records_seen += 1;

            if count >= limit || records_seen >= records_to_view_limit {
                next_record_key = Some(record_key);
                break;
            }

            let LastRecord {
                key: prev_key,
                value: prev_value,
            } = &mut last_record;

            let skip_collection_key = prev_value.is_none();

            let ParsedRecordKey {
                collection_key: prev_collection_key,
                generation_id: prev_generation_id,
                phantom_id: prev_phantom_id,
            } = prev_key.get_parsed();

            let ParsedRecordKey {
                collection_key: item_collection_key,
                generation_id: item_generation_id,
                phantom_id: item_phantom_id,
            } = record_key.get_parsed();

            let is_same_key = prev_collection_key == item_collection_key;

            if skip_collection_key {
                if is_same_key {
                    continue;
                }

                last_record = LastRecord {
                    key: record_key,
                    value: Some(value),
                };
                continue;
            }

            let is_found =
                !is_same_key || !is_generation_id_less_or_equal(item_generation_id, generation_id);

            if !is_found {
                if item_phantom_id == phantom_id {
                    last_record = LastRecord {
                        key: record_key,
                        value: Some(value),
                    };
                }
                continue;
            }

            // Need to check because we can have phantoms/older generations in `last_record`
            // at collection start or after collection_key skipping
            if prev_phantom_id == phantom_id
                && is_generation_id_less_or_equal(prev_generation_id, generation_id)
            {
                // `prev_value` cannot be None, because we are continuing if `skip_collection_key`
                push_to_result(
                    &mut result,
                    prev_collection_key,
                    OwnedCollectionValue::from_boxed_slice(prev_value.take().unwrap()),
                    &mut count,
                );
            }

            if is_same_key {
                // skip this key
                prev_value.take();
                continue;
            }

            // process next key
            last_record = LastRecord {
                key: record_key,
                value: Some(value),
            };
        }

        // There is two possible cases:
        //   - we are at the end of collection
        //   - we are touched limit (then `next_record_key` will be Some)

        let last_and_next_record_key = match next_record_key {
            Some(key) => {
                let last = OwnedRecordKey::from_owned_parsed_record_key(last_record.key);
                let next = OwnedRecordKey::from_owned_parsed_record_key(key);

                Some(LastAndNextRecordKey { last, next })
            }
            None => {
                (|| {
                    // In case of collection end we need maybe to push `last_record` to result
                    let LastRecord {
                        key: prev_key,
                        value: prev_value,
                    } = last_record;

                    let value = match prev_value {
                        Some(value) => value,
                        None => {
                            // this key was already pushed
                            return;
                        }
                    };

                    let ParsedRecordKey {
                        collection_key: prev_collection_key,
                        generation_id: prev_generation_id,
                        phantom_id: prev_phantom_id,
                    } = prev_key.get_parsed();

                    if prev_phantom_id != phantom_id
                        || !is_generation_id_less_or_equal(prev_generation_id, generation_id)
                    {
                        return;
                    }

                    push_to_result(
                        &mut result,
                        prev_collection_key,
                        OwnedCollectionValue::from_boxed_slice(value),
                        &mut count,
                    );
                })();
                None
            }
        };

        Ok(QueryCollectionRecordsResult {
            items: result,
            last_and_next_record_key,
        })
    }
}

fn push_to_result(
    result: &mut Vec<KeyValue>,
    key: CollectionKey<'_>,
    value: OwnedCollectionValue,
    count: &mut usize,
) {
    if value.is_empty() {
        return;
    }

    result.push(KeyValue {
        key: key.to_owned(),
        value,
    });

    *count += 1;
}

#[inline]
fn is_generation_id_less_or_equal(a: GenerationId<'_>, b: GenerationId<'_>) -> bool {
    a <= b
}
