use crate::collection::util::record_key::{OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKey};
use crate::common::{
    CollectionKey, GenerationId, IsByteArray, KeyValueDiff, OwnedCollectionKey,
    OwnedCollectionValue, PhantomId,
};
use crate::raw_db::diff_collection_records::state::{DiffState, PrevDiffState};
use crate::raw_db::diff_collection_records::{DiffCollectionRecordsOk, DiffCursorState};
use crate::raw_db::RawDbError;
use crate::util::owned_peek::OwnedPeek;
use rocksdb::{DBIterator, Direction, IteratorMode};

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
    ) -> DiffCollectionRecordsResult {
        let DiffState {
            db,
            from_generation_id,
            to_generation_id,
            records_to_view_left,
            pack_limit,
            prev_state,
        } = self;

        let capacity = items_capacity_hint.map_or(*pack_limit, |hint| Ord::min(*pack_limit, hint));
        let mut items = Vec::with_capacity(capacity);

        let mut changed_keys_iterator = OwnedPeek::new(changed_items_iterator);
        let (mut db_iterator, mut db_next_item) = match prev_state.take() {
            Some(PrevDiffState {
                first_value,
                last_value,
                next_record_key,
            }) => {
                let iterator_mode =
                    IteratorMode::From(next_record_key.get_byte_array(), Direction::Forward);

                let mut db_iterator = db.iterator(iterator_mode);

                let (record_key, value) =
                    db_iterator_parse_next_require_presense(&mut db_iterator)?;

                if record_key.get_collection_key() != next_record_key.get_collection_key() {
                    return Err(RawDbError::DiffNoChangedKeyRecord);
                }

                (
                    db_iterator,
                    KeyProcessing {
                        record_key,
                        value,
                        first_value: first_value.map(|bytes| bytes.into()),
                        last_value: last_value.map(|bytes| bytes.into()),
                    },
                )
            }
            None => {
                enum PeekResult<'a> {
                    Continue((DBIterator<'a>, KeyProcessing)),
                    Finish(DiffCollectionRecordsResult),
                    FinishEmpty,
                }

                let result: PeekResult<'_> = changed_keys_iterator.peek(|changed_key| {
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

                        let mut db_iterator = iterator_mode_for_collection_key(
                            changed_key.as_ref(),
                            |iterator_mode| db.iterator(iterator_mode),
                        )?;

                        let (record_key, value) =
                            db_iterator_parse_next_require_presense(&mut db_iterator)?;

                        if record_key.get_collection_key() != changed_key.as_ref() {
                            return Err(RawDbError::DiffNoChangedKeyRecord);
                        }

                        Ok((
                            (PeekResult::Continue((
                                db_iterator,
                                KeyProcessing {
                                    record_key,
                                    value,
                                    first_value: None,
                                    last_value: None,
                                },
                            ))),
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
                iterator_mode_for_collection_key(changed_key.as_ref(), |iterator_mode| {
                    db_iterator.set_mode(iterator_mode);
                })?;

                let (record_key, value) =
                    db_iterator_parse_next_require_presense(&mut db_iterator)?;

                if record_key.get_collection_key() != changed_key.as_ref() {
                    return Err(RawDbError::DiffNoChangedKeyRecord);
                }

                KeyProcessing {
                    record_key,
                    value,
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
                                changed_key: record_key.get_collection_key().to_owned(),
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

                // Process current key
                for result in db_iterator.by_ref() {
                    let (key, value): (Box<[u8]>, Box<[u8]>) = result?;

                    let record_key = OwnedParsedRecordKey::from_boxed_slice(key)
                        .or(Err(RawDbError::InvalidRecordKey))?;

                    match handle_db_record(record_key, value) {
                        HandleDbRecordResult::CollectionKeyChanged(item) => {
                            next_item = Some(item);
                            break;
                        }
                        HandleDbRecordResult::Finish(record_key) => {
                            return Ok(DiffCollectionRecordsOk {
                                to_generation_id: to_generation_id.clone(),
                                items,
                                next_diff_state: Some(DiffCursorState {
                                    changed_key: record_key.get_collection_key().to_owned(),
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
fn iterator_mode_for_collection_key<T>(
    key: CollectionKey<'_>,
    fun: impl FnOnce(IteratorMode<'_>) -> T,
) -> Result<T, RawDbError> {
    let record_key = OwnedRecordKey::new(key, GenerationId::empty(), PhantomId::empty())
        .or(Err(RawDbError::InvalidRecordKey))?;

    Ok(fun(IteratorMode::From(
        record_key.get_byte_array(),
        Direction::Forward,
    )))
}

type DbIteratorItem = Result<(Box<[u8]>, Box<[u8]>), rocksdb::Error>;

fn db_iterator_maybe_parse_next(
    mut db_iterator: impl Iterator<Item = DbIteratorItem>,
) -> Result<Option<(OwnedParsedRecordKey, Box<[u8]>)>, RawDbError> {
    let result = db_iterator.next();
    let (key, value) = match result {
        Some(result) => result?,
        None => {
            return Ok(None);
        }
    };

    let record_key =
        OwnedParsedRecordKey::from_boxed_slice(key).or(Err(RawDbError::InvalidRecordKey))?;

    Ok(Some((record_key, value)))
}

fn db_iterator_parse_next_require_presense(
    db_iterator: impl Iterator<Item = DbIteratorItem>,
) -> Result<(OwnedParsedRecordKey, Box<[u8]>), RawDbError> {
    db_iterator_maybe_parse_next(db_iterator)?.ok_or(RawDbError::DiffNoChangedKeyRecord)
}
