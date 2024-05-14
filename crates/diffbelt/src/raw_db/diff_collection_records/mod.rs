use std::cmp;

use rocksdb::WriteBatch;

use crate::collection::constants::{COLLECTION_CF_META, COLLECTION_META_GC_PHANTOM_ID_KEY};
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{
    GenerationId, IsByteArray, KeyValueDiff, OwnedCollectionKey, OwnedGenerationId, PhantomId,
};
use crate::raw_db::diff_collection_records::state::changed_keys_iterator::ChangedKeysIteratorImpl;
use crate::raw_db::diff_collection_records::state::in_memory::InMemoryChangedKeysIter;
use crate::raw_db::diff_collection_records::state::single_generation::SingleGenerationChangedKeysIter;
use crate::raw_db::diff_collection_records::state::{DiffState, DiffStateMode, DiffStateNewResult};
use crate::raw_db::diff_logic::input::DiffLogicInput;
use crate::raw_db::diff_logic::{
    DiffLogicAction, DiffLogicSubAction, EmitItemAction, NewDiffLogic, ResumeDiffLogic,
};
use crate::raw_db::garbage_collector::gc_iterator::NewGcIterator;
use crate::raw_db::{RawDb, RawDbError};

mod state;

pub struct DiffCollectionRecordsOptions<'a> {
    pub from_generation_id: Option<GenerationId<'a>>,
    // Not loose if `prev_diff_state` is specified
    pub to_generation_id_loose: GenerationId<'a>,
    pub prev_diff_state: Option<&'a DiffCursorState>,
    pub limit: usize,
    pub records_to_view_limit: usize,
    pub total_count_in_generations_limit: usize,
}

pub struct DiffCollectionRecordsOk {
    pub to_generation_id: OwnedGenerationId,
    pub items: Vec<KeyValueDiff>,
    pub next_diff_state: Option<DiffCursorState>,
}

#[derive(Debug)]
pub struct DiffCursorState {
    changed_key: Option<OwnedCollectionKey>,
    diff_logic_resume: ResumeDiffLogic,
    seek_cursor_to: Option<OwnedRecordKey>,
}

impl RawDb {
    pub fn diff_collection_records_sync(
        &self,
        options: DiffCollectionRecordsOptions<'_>,
    ) -> Result<DiffCollectionRecordsOk, RawDbError> {
        let DiffCollectionRecordsOptions {
            from_generation_id,
            to_generation_id_loose,
            prev_diff_state,
            limit,
            mut records_to_view_limit,
            total_count_in_generations_limit,
        } = options;

        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let gc_phantom_id = db.get_pinned_cf(&meta_cf, COLLECTION_META_GC_PHANTOM_ID_KEY)?;
        let gc_phantom_id = gc_phantom_id
            .as_ref()
            .map(|x| PhantomId::new_unchecked(x.as_ref()));

        let state = match prev_diff_state {
            Some(prev_state) => DiffState::continue_prev(
                db,
                from_generation_id,
                to_generation_id_loose,
                prev_state,
                limit,
                records_to_view_limit,
            )?,
            None => DiffState::new(
                db,
                from_generation_id,
                to_generation_id_loose,
                limit,
                records_to_view_limit,
                total_count_in_generations_limit,
            )?,
        };

        let (mut state, mode) = match state {
            DiffStateNewResult::Empty => {
                return Ok(DiffCollectionRecordsOk {
                    to_generation_id: to_generation_id_loose.to_owned(),
                    items: Vec::with_capacity(0),
                    next_diff_state: None,
                });
            }
            DiffStateNewResult::State(x) => x,
        };

        let mut keys_to_remove = Vec::new();

        let to_generation_id = state.to_generation_id.clone();

        let logic = NewDiffLogic {
            from_generation_id,
            to_generation_id: to_generation_id.as_ref(),
            items_limit: limit,
        };
        let mut logic = match prev_diff_state {
            None => logic.new(),
            Some(continuation) => logic.resume(&continuation.diff_logic_resume),
        };

        let (capacity_hint, mut changed_keys_iterator) = match mode {
            DiffStateMode::InMemory(in_memory) => {
                let capacity_hint = in_memory.changed_keys.len();
                let iterator = InMemoryChangedKeysIter::new(in_memory.changed_keys);

                (capacity_hint, ChangedKeysIteratorImpl::InMemory(iterator))
            }
            DiffStateMode::SingleGeneration => {
                let iterator = SingleGenerationChangedKeysIter::new(
                    db,
                    state.get_to_generation_id(),
                    state.get_from_collection_key(),
                )?;

                (limit, ChangedKeysIteratorImpl::SingleGeneration(iterator))
            }
        };

        let mut items = Vec::with_capacity(cmp::min(limit, capacity_hint));

        let mut gc_iterator = NewGcIterator {
            gc_phantom_id,
            db_iterator: db.raw_iterator(),
            keys_to_delete: &mut keys_to_remove,
        }
        .new();

        if let Some(continuation) = prev_diff_state {
            let key = continuation.seek_cursor_to.as_ref().map(|x| x.as_ref());
            if let Some(key) = key {
                () = gc_iterator.seek(key.get_byte_array())?;
            } else {
                gc_iterator.make_invalid();
            }
        }

        let mut action = logic.run(DiffLogicInput::Init)?;
        let mut continuation = None;
        let mut seek_cursor_to = None;

        loop {
            let sub = match action {
                DiffLogicAction::GetNextChangedKey => {
                    let key = changed_keys_iterator.next().transpose()?;
                    let key = key.as_ref().map(|x| x.as_ref());

                    records_to_view_limit = records_to_view_limit.checked_sub(1).unwrap_or(0);
                    if records_to_view_limit == 0 {
                        logic.require_to_stop();
                    }

                    action = logic.run(DiffLogicInput::NextChangedKey(key))?;
                    continue;
                }
                DiffLogicAction::SetCursorAndGetNext(key) => {
                    gc_iterator.seek(key.get_byte_array())?;
                    let key = gc_iterator.next_key()?;

                    records_to_view_limit = records_to_view_limit.checked_sub(1).unwrap_or(0);
                    if records_to_view_limit == 0 {
                        logic.require_to_stop();
                    }

                    action = logic.run(DiffLogicInput::NextCursorKey(key))?;
                    continue;
                }
                DiffLogicAction::GetNextCursorKey => {
                    let key = gc_iterator.next_key()?;

                    records_to_view_limit = records_to_view_limit.checked_sub(1).unwrap_or(0);
                    if records_to_view_limit == 0 {
                        logic.require_to_stop();
                    }

                    action = logic.run(DiffLogicInput::NextCursorKey(key))?;
                    continue;
                }
                DiffLogicAction::EmitItem(EmitItemAction {
                    key,
                    from_key,
                    to_key,
                    next_action,
                }) => {
                    () = gc_iterator.save_state()?;

                    let from_value = from_key
                        .map(|x| gc_iterator.get_value_for_key(x))
                        .transpose()?
                        .and_then(|x| x.to_owned_if_not_empty());
                    let to_value = to_key
                        .map(|x| gc_iterator.get_value_for_key(x))
                        .transpose()?;

                    let is_changed = from_value
                        .as_ref()
                        .map(|x| x.as_ref())
                        .and_then(|x| x.to_none_if_empty())
                        != to_value.and_then(|x| x.to_none_if_empty());

                    if is_changed {
                        let to_value = to_value.and_then(|x| x.to_owned_if_not_empty());

                        items.push(KeyValueDiff {
                            key: key.to_owned(),
                            from_value,
                            intermediate_values: Vec::new(),
                            to_value,
                        });
                    }

                    () = gc_iterator.restore_state()?;

                    next_action
                }
                DiffLogicAction::Sub(sub) => sub,
            };

            match sub {
                DiffLogicSubAction::SetCursorAndGetNextCursorKeyAndNextChangedKey(key) => {
                    gc_iterator.seek(key.get_byte_array())?;
                    let cursor_key = gc_iterator.next_key()?;
                    let changed_key = changed_keys_iterator.next().transpose()?;
                    let changed_key = changed_key.as_ref().map(|x| x.as_ref());

                    records_to_view_limit = records_to_view_limit.checked_sub(1).unwrap_or(0);
                    if records_to_view_limit == 0 {
                        logic.require_to_stop();
                    }

                    action = logic.run(DiffLogicInput::NextCursorAndChangedKey((
                        cursor_key,
                        changed_key,
                    )))?;
                    continue;
                }
                DiffLogicSubAction::GetNextCursorKeyAndNextChangedKey => {
                    let cursor_key = gc_iterator.next_key()?;
                    let changed_key = changed_keys_iterator.next().transpose()?;
                    let changed_key = changed_key.as_ref().map(|x| x.as_ref());

                    records_to_view_limit = records_to_view_limit.checked_sub(1).unwrap_or(0);
                    if records_to_view_limit == 0 {
                        logic.require_to_stop();
                    }

                    action = logic.run(DiffLogicInput::NextCursorAndChangedKey((
                        cursor_key,
                        changed_key,
                    )))?;
                    continue;
                }
                DiffLogicSubAction::Finish(cont) => {
                    continuation = cont;

                    let cursor_key = gc_iterator.next_key()?;
                    seek_cursor_to = cursor_key.map(|x| x.as_record_key().to_owned());

                    break;
                }
            }
        }

        drop(gc_iterator);

        let next_diff_state = match continuation {
            None => None,
            Some(diff_logic_resume) => {
                let changed_key = changed_keys_iterator.next().transpose()?;

                Some(DiffCursorState {
                    changed_key,
                    diff_logic_resume,
                    seek_cursor_to,
                })
            }
        };

        if !keys_to_remove.is_empty() {
            let mut batch = WriteBatch::default();

            for record_key in keys_to_remove.drain(..) {
                batch.delete(record_key.get_byte_array());
            }

            db.write(batch)?;
        }

        Ok(DiffCollectionRecordsOk {
            to_generation_id,
            items,
            next_diff_state,
        })
    }
}
