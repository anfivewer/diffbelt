use crate::collection::util::record_key::{OwnedRecordKey, RecordKey};
use crate::common::{CollectionKey, GenerationId, KeyValue, OwnedCollectionValue, PhantomId};

use crate::raw_db::query::{
    ContinuationState, QueryDirectionForward, QueryKeyValue, QueryOptions, QueryState,
};
use crate::raw_db::{RawDb, RawDbError};

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

        let db = self.db.get_db();

        let mut count = 0usize;
        let mut result = Vec::with_capacity(limit);

        let mut query = QueryState::new(
            db,
            QueryOptions {
                kind: QueryKeyValue,
                direction: QueryDirectionForward,
                start_key: from_record_key.as_ref().map(|x| x.get_collection_key()),
                generation_id,
                phantom_id,
                continuation_state: last_record_key
                    .as_ref()
                    .map(|last_record| ContinuationState {
                        last_candidate_key: last_record.to_owned(),
                        next_iterator_key: from_record_key.as_ref().unwrap().to_owned(),
                    }),
                records_to_view_limit,
            },
        )?;

        for item in query.by_ref() {
            let item = item?;

            result.push(KeyValue {
                key: item.key.get_collection_key().to_owned(),
                value: OwnedCollectionValue::from_boxed_slice(item.value),
            });

            count += 1;

            if count >= limit {
                break;
            }
        }

        let continuation = query.into_continuation();

        Ok(QueryCollectionRecordsResult {
            items: result,
            last_and_next_record_key: continuation.map(|continuation| LastAndNextRecordKey {
                last: continuation.last_candidate_key,
                next: continuation.next_iterator_key,
            }),
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
