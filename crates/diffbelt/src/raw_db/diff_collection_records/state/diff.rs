use rocksdb::{DBIterator, Direction, IteratorMode, ReadOptions};

use crate::collection::util::record_key::{OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKey};
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
        let DiffState {
            db,
            from_generation_id,
            to_generation_id,
            records_to_view_left,
            pack_limit,
            prev_state,
        } = self;

        let mut pack_size_left = *pack_limit;

        let capacity = items_capacity_hint.map_or(*pack_limit, |hint| Ord::min(*pack_limit, hint));
        let mut items = Vec::with_capacity(capacity);

        let mut changed_keys_iterator = OwnedPeek::new(changed_items_iterator);
        let (mut db_iterator, mut db_next_item) = match prev_state.take() {
            Some(PrevDiffState {
                first_value,
                last_value,
                next_record_key,
            }) => {
                let mut iterator_opts = ReadOptions::default();
                iterator_opts.set_iterate_lower_bound(next_record_key.get_byte_array());

                let db_iterator = db.raw_iterator_opt(iterator_opts);
                let mut db_iterator = NewGcIterator {
                    gc_phantom_id,
                    db_iterator,
                    keys_to_delete,
                }
                .new();

                let (record_key, value) =
                    db_iterator_parse_next_require_presense(&mut db_iterator)?;

                if record_key.collection_key != next_record_key.get_collection_key() {
                    return Err(RawDbError::DiffNoChangedKeyRecord);
                }

                let key_processing = KeyProcessing {
                    record_key: OwnedParsedRecordKey::from_owned_record_key(OwnedRecordKey::from(
                        record_key,
                    )),
                    value: Box::from(value),
                    first_value: first_value.map(|bytes| bytes.into()),
                    last_value: last_value.map(|bytes| bytes.into()),
                };

                (db_iterator, key_processing)
            }
            None => {
                enum PeekResult<'a> {
                    Continue((GcIterator<'a>, KeyProcessing)),
                    Finish(DiffCollectionRecordsResult),
                    FinishEmpty,
                }

                let result: PeekResult = changed_keys_iterator.peek(|changed_key| {
                    let result = (|| {
                        let changed_key = match changed_key {
                            Some(result) => match result {
                                Ok(key) => key,
                                Err(err) => {
                                    return Err(err);
                                }
                            },
                            None => {
                                return Ok(((PeekResult::FinishEmpty), None));
                            }
                        };

                        let iterator_opts = iterator_opts_for_collection_key(changed_key.as_ref())?;
                        let mut db_iterator = NewGcIterator {
                            gc_phantom_id,
                            db_iterator: db.raw_iterator_opt(iterator_opts),
                            keys_to_delete,
                        }
                        .new();

                        let key_processing = {
                            let (record_key, value) = db_iterator
                                .next()?
                                .ok_or(RawDbError::DiffNoChangedKeyRecord)?;

                            if record_key.collection_key != changed_key.as_ref() {
                                return Err(RawDbError::DiffNoChangedKeyRecord);
                            }

                            let record_key = OwnedParsedRecordKey::from_owned_record_key(
                                OwnedRecordKey::from(record_key),
                            );
                            let value = Box::from(value);

                            KeyProcessing {
                                record_key,
                                value,
                                first_value: None,
                                last_value: None,
                            }
                        };

                        Ok((
                            PeekResult::Continue((db_iterator, key_processing)),
                            Some(Ok(changed_key)),
                        ))
                    })();

                    match result {
                        Ok((result, value)) => (result, value),
                        Err(err) => (PeekResult::Finish(Err(err)), None),
                    }
                });

                match result {
                    PeekResult::Continue(result) => result,
                    PeekResult::Finish(result) => {
                        return result;
                    }
                    PeekResult::FinishEmpty => {
                        return Ok(DiffCollectionRecordsOk {
                            to_generation_id: to_generation_id.clone(),
                            items,
                            next_diff_state: None,
                        });
                    }
                }
            }
        };

        loop {
            let changed_key = changed_keys_iterator.next();
            let changed_key = match changed_key {
                Some(result) => result?,
                None => {
                    break;
                }
            };

            // Find record where `collection_key == changed_key`
            // there is little optimization for keys that are going consequentially,
            // we are not doing jumps for them
            let record_key = db_next_item.record_key.get_parsed();

            let KeyProcessing {
                record_key,
                value,
                mut first_value,
                mut last_value,
            } = if record_key.collection_key != changed_key.as_ref() {
                // jump to required key
                let iterator_opts = iterator_opts_for_collection_key(changed_key.as_ref())?;
                let keys_to_delete = db_iterator.into_keys_to_delete();
                db_iterator = NewGcIterator {
                    gc_phantom_id,
                    db_iterator: db.raw_iterator_opt(iterator_opts),
                    keys_to_delete,
                }
                .new();

                let (record_key, value) =
                    db_iterator_parse_next_require_presense(&mut db_iterator)?;

                if record_key.collection_key != changed_key.as_ref() {
                    return Err(RawDbError::DiffNoChangedKeyRecord);
                }

                KeyProcessing {
                    record_key: OwnedParsedRecordKey::from_owned_record_key(OwnedRecordKey::from(
                        record_key,
                    )),
                    value: Box::from(value),
                    first_value: None,
                    last_value: None,
                }
            } else {
                db_next_item
            };

            let mut handle_db_record = |record_key: OwnedParsedRecordKey, value: Box<[u8]>| {
                let ParsedRecordKey {
                    collection_key,
                    generation_id,
                    phantom_id,
                } = record_key.get_parsed();

                *records_to_view_left -= 1;

                if *records_to_view_left <= 0 {
                    return HandleDbRecordResult::Finish(record_key);
                }

                if collection_key != changed_key.as_ref() {
                    return HandleDbRecordResult::CollectionKeyChanged((record_key, value));
                }
                if phantom_id.is_some() || generation_id > (*to_generation_id).as_ref() {
                    return HandleDbRecordResult::Continue;
                }

                // If `from_generation_id` is None, `first_value` should be None
                if generation_id.less_or_equal_with_opt_or(*from_generation_id, false) {
                    first_value = Some(value);
                } else {
                    last_value = Some(value);
                }

                HandleDbRecordResult::Continue
            };

            db_next_item = {
                match handle_db_record(record_key, value) {
                    HandleDbRecordResult::CollectionKeyChanged(_) => {
                        return Err(RawDbError::DiffNoChangedKeyRecord);
                    }
                    HandleDbRecordResult::Finish(record_key) => {
                        return Ok(DiffCollectionRecordsOk {
                            to_generation_id: to_generation_id.clone(),
                            items,
                            next_diff_state: Some(DiffCursorState {
                                changed_key,
                                first_value,
                                last_value,
                                next_record_key: OwnedRecordKey::from_owned_parsed_record_key(
                                    record_key,
                                ),
                            }),
                        });
                    }
                    HandleDbRecordResult::Continue => {}
                }

                let mut next_item: Option<RecordKeyWithValue> = None;

                db_iterator.save_state()?;

                // Process current key
                while let Some((key, value)) = db_iterator.next()? {
                    let record_key =
                        OwnedParsedRecordKey::from_owned_record_key(OwnedRecordKey::from(key));

                    match handle_db_record(record_key, Box::from(value)) {
                        HandleDbRecordResult::CollectionKeyChanged(item) => {
                            next_item = Some(item);
                            break;
                        }
                        HandleDbRecordResult::Finish(record_key) => {
                            return Ok(DiffCollectionRecordsOk {
                                to_generation_id: to_generation_id.clone(),
                                items,
                                next_diff_state: Some(DiffCursorState {
                                    changed_key,
                                    first_value,
                                    last_value,
                                    next_record_key: OwnedRecordKey::from_owned_parsed_record_key(
                                        record_key,
                                    ),
                                }),
                            });
                        }
                        HandleDbRecordResult::Continue => {}
                    }
                }

                db_iterator.restore_state()?;

                match next_item {
                    // There `record_key` collection_key != changed_key
                    Some((record_key, value)) => KeyProcessing {
                        record_key,
                        value,
                        first_value: None,
                        last_value: None,
                    },
                    None => {
                        // End of iterator
                        if !changed_keys_iterator.is_empty() {
                            return Err(RawDbError::DiffNoChangedKeyRecord);
                        }

                        handle_item(
                            changed_key.as_ref(),
                            &mut items,
                            &mut first_value,
                            &mut last_value,
                        )?;

                        return Ok(DiffCollectionRecordsOk {
                            to_generation_id: to_generation_id.clone(),
                            items,
                            next_diff_state: None,
                        });
                    }
                }
            };

            handle_item(
                changed_key.as_ref(),
                &mut items,
                &mut first_value,
                &mut last_value,
            )?;
            pack_size_left -= 1;

            if pack_size_left <= 0 {
                // Since we are pushed this item, we need to save next key in the cursor
                let changed_key = lift_result_from_option(changed_keys_iterator.next())?;

                let KeyProcessing {
                    record_key,
                    value: _,
                    first_value,
                    last_value,
                } = db_next_item;

                return Ok(DiffCollectionRecordsOk {
                    to_generation_id: to_generation_id.clone(),
                    items,
                    next_diff_state: changed_key.map(|changed_key| DiffCursorState {
                        changed_key,
                        first_value,
                        last_value,
                        next_record_key: OwnedRecordKey::from_owned_parsed_record_key(record_key),
                    }),
                });
            }
        }

        Ok(DiffCollectionRecordsOk {
            to_generation_id: to_generation_id.clone(),
            items,
            next_diff_state: None,
        })
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
) -> Result<(ParsedRecordKey<'b>, &'b [u8]), RawDbError> {
    db_iterator
        .next()?
        .ok_or(RawDbError::DiffNoChangedKeyRecord)
}
