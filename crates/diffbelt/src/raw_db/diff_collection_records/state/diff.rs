use rocksdb::{DBIterator, Direction, IteratorMode, ReadOptions};

use crate::collection::util::record_key::{OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKeyOld};
use crate::common::{
    CollectionKey, GenerationId, IsByteArray, KeyValueDiff, OwnedCollectionKey,
    OwnedCollectionValue, PhantomId,
};
use crate::raw_db::diff_collection_records::state::{DiffState, PrevDiffState};
use crate::raw_db::diff_collection_records::{DiffCollectionRecordsOk, DiffCursorState};
use crate::raw_db::garbage_collector::gc_iterator::{GcIterator, NewGcIterator};
use crate::raw_db::RawDbError;
use crate::util::option::lift_result_from_option;
use crate::util::owned_peek::OwnedPeek;

struct KeyProcessing {
    record_key: OwnedParsedRecordKey,
    value: Box<[u8]>,
    first_value: Option<Box<[u8]>>,
    last_value: Option<Box<[u8]>>,
}

type RecordKeyWithValue = (OwnedParsedRecordKey, Box<[u8]>);

enum HandleDbRecordResult {
    CollectionKeyChanged(RecordKeyWithValue),
    Finish(OwnedParsedRecordKey),
    Continue,
}

type DiffCollectionRecordsResult = Result<DiffCollectionRecordsOk, RawDbError>;

impl DiffState<'_> {
    pub fn diff_collection_records_sync(
        &mut self,
        changed_items_iterator: impl Iterator<Item = Result<OwnedCollectionKey, RawDbError>>,
        items_capacity_hint: Option<usize>,
        keys_to_delete: &mut Vec<OwnedRecordKey>,
        gc_phantom_id: Option<PhantomId<'_>>,
    ) -> DiffCollectionRecordsResult {
        panic!("to remove")
    }
}

fn handle_item(
    collection_key: CollectionKey<'_>,
    items: &mut Vec<KeyValueDiff>,
    first_value: &mut Option<Box<[u8]>>,
    last_value: &mut Option<Box<[u8]>>,
) -> Result<(), RawDbError> {
    items.push(KeyValueDiff {
        key: collection_key.to_owned(),
        from_value: first_value
            .take()
            .and_then(|bytes| OwnedCollectionValue::from_boxed_slice_opt(bytes)),
        intermediate_values: Vec::with_capacity(0),
        to_value: last_value
            .take()
            .and_then(|bytes| OwnedCollectionValue::from_boxed_slice_opt(bytes)),
    });

    Ok(())
}

#[inline]
fn iterator_opts_for_collection_key(key: CollectionKey<'_>) -> Result<ReadOptions, RawDbError> {
    let record_key = OwnedRecordKey::new(key, GenerationId::empty(), PhantomId::empty())
        .or(Err(RawDbError::InvalidRecordKey))?;

    let mut iterator_opts = ReadOptions::default();
    iterator_opts.set_iterate_lower_bound(record_key.get_byte_array());

    Ok(iterator_opts)
}

fn db_iterator_parse_next_require_presense<'b, 'a>(
    db_iterator: &'b mut GcIterator<'a>,
) -> Result<(ParsedRecordKeyOld<'b>, &'b [u8]), RawDbError> {
    db_iterator
        .next()?
        .ok_or(RawDbError::DiffNoChangedKeyRecord("db_iterator_parse_next_require_presense"))
}
