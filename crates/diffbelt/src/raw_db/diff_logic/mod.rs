use crate::collection::util::record_key::{ParsedRecordKey, ParsedRecordKeyRanges, RecordKey};
use crate::common::{GenerationId, IsByteArray};
use crate::raw_db::diff_logic::error::DiffLogicError;
use crate::raw_db::diff_logic::input::DiffLogicInput;
use std::mem;

pub mod error;
mod input;

pub struct EmitItemAction<'a> {
    pub from_key: RecordKey<'a>,
    pub to_key: RecordKey<'a>,
    pub next_action: DiffLogicSubAction<'a>,
}

pub enum DiffLogicSubAction<'a> {
    SetCursorAndGetNextCursorKeyAndNextChangedKey(RecordKey<'a>),
    GetNextCursorKeyAndNextChangedKey,
    Finish,
}

pub enum DiffLogicAction<'a> {
    GetNextChangedKey,
    SetCursorAndGetNext(RecordKey<'a>),
    GetNextCursorKey,
    EmitItem(EmitItemAction<'a>),
    Sub(DiffLogicSubAction<'a>),
}

type ParsedRecordKeyHolder = (ParsedRecordKeyRanges, Vec<u8>);
type ParsedRecordKeyHolderOptional = (Option<ParsedRecordKeyRanges>, Vec<u8>);

type HandlerResult<'a> = Result<DiffLogicAction<'a>, DiffLogicError>;

pub struct DiffLogic<'a> {
    from_generation_id: GenerationId<'a>,
    to_generation_id: GenerationId<'a>,
    items_limit: usize,
    records_to_view_limit: usize,
    handler: for<'b, 'c: 'b> fn(
        &'c mut Self,
        DiffLogicInput<'b>,
    ) -> Result<DiffLogicAction<'c>, DiffLogicError>,
    changed_key_holder: ParsedRecordKeyHolder,
    next_changed_key_holder: ParsedRecordKeyHolderOptional,
    current_record_key_holder: ParsedRecordKeyHolder,
    last_candidate_key_holder: ParsedRecordKeyHolderOptional,
    first_key_holder: ParsedRecordKeyHolderOptional,
    last_key_holder: ParsedRecordKeyHolderOptional,

    temp_key_holder: ParsedRecordKeyHolder,
}

impl DiffLogic<'_> {
    pub fn run(&mut self, input: DiffLogicInput) -> HandlerResult<'_> {
        self.handler(input)
    }

    fn init(&mut self) -> Result<DiffLogicAction, DiffLogicError> {
        Ok(DiffLogicAction::GetNextChangedKey)
    }

    fn on_finish(&mut self) -> HandlerResult<'_> {
        Err(DiffLogicError::AlreadyFinished)
    }

    fn on_got_first_changed_key(
        &mut self,
        changed_key: Option<ParsedRecordKey<'_>>,
    ) -> HandlerResult<'_> {
        let Some(changed_key) = changed_key else {
            return Ok(DiffLogicAction::Sub(DiffLogicSubAction::Finish));
        };

        write_key_to_holder(&mut self.changed_key_holder, &changed_key);

        Ok(DiffLogicAction::GetNextChangedKey)
    }

    fn on_got_next_changed_key(
        &mut self,
        changed_key: Option<ParsedRecordKey<'_>>,
    ) -> HandlerResult<'_> {
        write_key_to_holder_optional(&mut self.next_changed_key_holder, changed_key.as_ref());

        // Start from empty generation to find first value
        let key_with_no_generation = ParsedRecordKey::new_on_vec(
            &mut self.temp_key_holder.1,
            changed_key.collection_key(),
            GenerationId::empty(),
            None,
        )
        .map_err(|()| DiffLogicError::InvalidRecordKey)?;

        self.temp_key_holder.0 = key_with_no_generation.ranges().clone();

        let key_with_no_generation = record_key_from_holder(&self.temp_key_holder);

        Ok(DiffLogicAction::SetCursorAndGetNext(key_with_no_generation))
    }

    fn on_got_next_after_cursor_set(
        &mut self,
        key: Option<ParsedRecordKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        let Some(key) = key else {
            return Err(DiffLogicError::ChangedKeyNotFound);
        };

        let changed_key = parsed_key_from_holder(&self.changed_key_holder);

        if key.collection_key() != changed_key.collection_key() {
            return Err(DiffLogicError::Unspecified(
                "First iterator key is not changed one".to_string(),
            ));
        }

        write_key_to_holder(&mut self.current_record_key_holder, &key);

        Ok(DiffLogicAction::GetNextCursorKey)
    }

    fn on_got_next_cursor(
        &mut self,
        key: Option<ParsedRecordKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        self.handle_keys_pair(key)
    }

    fn handle_keys_pair(
        &mut self,
        next_key: Option<ParsedRecordKey<'_>>,
    ) -> Result<DiffLogicAction<'_>, DiffLogicError> {
        let current_key = parsed_key_from_holder(&self.current_record_key_holder);
        let current_collection_key = current_key.collection_key();

        // If this is last key, or collection key differs, or next generation is greater than we
        // need, then this is the end for this key
        let is_last_key = next_key.as_ref().map_or(true, |key| {
            key.collection_key() != current_collection_key
                || key.generation_id() > self.to_generation_id
        });

        let mut is_next_key_is_new_first = false;
        let mut is_next_key_is_new_last = false;

        let current_key_is_phantom = current_key.phantom_id().is_some();

        // View for next key, maybe no need to copy current key to first/last yet
        // No need to do it if current key is phantom, it will not be written
        if !is_last_key && !current_key_is_phantom {
            if let Some(next_key) = next_key {
                let generation_id = next_key.generation_id();

                if generation_id <= self.from_generation_id {
                    is_next_key_is_new_first = true;
                }
                if generation_id <= self.to_generation_id {
                    is_next_key_is_new_last = true;
                }
            }
        }

        // Write current key to first/last holders
        if !current_key_is_phantom {
            let generation_id = current_key.generation_id();

            if !is_next_key_is_new_first && generation_id <= self.from_generation_id {
                write_key_to_holder_optional(&mut self.first_key_holder, Some(&current_key));
            }
            if !is_next_key_is_new_last && generation_id <= self.to_generation_id {
                write_key_to_holder_optional(&mut self.last_key_holder, Some(&current_key));
            }
        }

        if !is_last_key {
            // Store next key as current and continue
            write_key_to_holder(
                &mut self.current_record_key_holder,
                next_key.as_ref().expect("already checked"),
            );

            return Ok(DiffLogicAction::GetNextCursorKey);
        }

        // This is last key. In first/current holders we now should contain keys before
        // from_generation_id and to_generation_id.

        let first_key = parsed_key_from_holder_optional(&self.first_key_holder);
        let last_key = parsed_key_from_holder_optional(&self.last_key_holder);

        let (first_key, last_key) = match (first_key, last_key) {
            (Some(first_key), Some(last_key)) => (first_key, last_key),
            _ => {
                // This should not happen, since we are scanning all keys and changed key is
                // pointing to some existing key
                return Err(DiffLogicError::NoFirstAndLastKeys);
            }
        };

        if first_key == last_key {
            // It is possible only if from_generation_id == to_generation_id, but it should be
            // checked before diff
            return Err(DiffLogicError::ChangeNotFound);
        }

        let next_changed_key = move_parsed_key_from_optional_to_holder(
            &mut self.next_changed_key_holder,
            &mut self.changed_key_holder,
        );

        let next_action = 'block: {
            let Some(next_changed_key) = next_changed_key.as_ref() else {
                // No more changed keys to see
                break 'block DiffLogicSubAction::Finish;
            };

            if let Some(next_key) = next_key.as_ref() {
                if next_key.collection_key() == next_changed_key.collection_key() {
                    // If next changed key is following current key, no need to set cursor to it,
                    // just continue traversing
                    break 'block DiffLogicSubAction::GetNextCursorKeyAndNextChangedKey;
                }
            }

            DiffLogicSubAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(
                next_changed_key.as_record_key(),
            )
        };

        Ok(
            EmitItemAction::EmitItemSetCursorAndGetNextAndNextChangedKey(EmitItemAction {
                from_key: first_key.as_record_key(),
                to_key: last_key.as_record_key(),
                next_action,
            }),
        )
    }

    // TODO
}

fn parsed_key_from_holder(holder: &ParsedRecordKeyHolder) -> ParsedRecordKey<'_> {
    let (parts, holder) = holder;
    ParsedRecordKey::new_unchecked(&holder, parts.clone())
}

fn parsed_key_from_holder_optional(
    holder: &ParsedRecordKeyHolderOptional,
) -> Option<ParsedRecordKey<'_>> {
    let (parts, holder) = holder;

    let Some(parts) = parts else {
        return None;
    };

    Some(ParsedRecordKey::new_unchecked(&holder, parts.clone()))
}

fn move_parsed_key_from_optional_to_holder(
    from_holder: &mut ParsedRecordKeyHolderOptional,
    to_holder: &mut ParsedRecordKeyHolder,
) -> Option<ParsedRecordKey<'_>> {
    let (from_parts, from_holder) = from_holder;

    let Some(from_parts) = from_parts.take() else {
        return None;
    };

    let (to_parts, to_holder) = to_holder;

    *to_parts = from_parts;
    mem::swap(from_holder, to_holder);

    Some(ParsedRecordKey::new_unchecked(
        &*to_holder,
        to_parts.clone(),
    ))
}

fn record_key_from_holder(holder: &ParsedRecordKeyHolder) -> RecordKey<'_> {
    let (_, holder) = holder;
    RecordKey::new_unchecked(&holder)
}

fn write_key_to_holder(holder: &mut ParsedRecordKeyHolder, key: &ParsedRecordKey<'_>) {
    let (parts, holder) = holder;

    holder.clear();
    holder.extend_from_slice(key.bytes());
    *parts = key.ranges().clone();
}

fn write_key_to_holder_optional(
    holder: &mut ParsedRecordKeyHolderOptional,
    key: Option<&ParsedRecordKey<'_>>,
) {
    let (parts, holder) = holder;

    let Some(key) = key else {
        *parts = None;
        return;
    };

    holder.clear();
    holder.extend_from_slice(key.bytes());
    *parts = Some(key.ranges().clone());
}
