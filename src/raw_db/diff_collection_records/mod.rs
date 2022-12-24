use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{GenerationId, KeyValueDiff, OwnedCollectionKey, OwnedGenerationId};
use crate::raw_db::diff_collection_records::state::in_memory::InMemoryChangedKeysIter;
use crate::raw_db::diff_collection_records::state::single_generation::SingleGenerationChangedKeysIter;
use crate::raw_db::diff_collection_records::state::{DiffState, DiffStateMode, DiffStateNewResult};
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

pub struct DiffCursorState {
    changed_key: OwnedCollectionKey,
    first_value: Option<Box<[u8]>>,
    last_value: Option<Box<[u8]>>,
    next_record_key: OwnedRecordKey,
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
            records_to_view_limit,
            total_count_in_generations_limit,
        } = options;

        let state = match prev_diff_state {
            Some(prev_state) => DiffState::continue_prev(
                &self.db,
                from_generation_id,
                to_generation_id_loose,
                prev_state,
                limit,
                records_to_view_limit,
            )?,
            None => DiffState::new(
                &self.db,
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

        match mode {
            DiffStateMode::InMemory(in_memory) => {
                let capacity_hint = Some(in_memory.changed_keys.len());
                let iterator = InMemoryChangedKeysIter::new(in_memory.changed_keys);

                state.diff_collection_records_sync(iterator, capacity_hint)
            }
            DiffStateMode::SingleGeneration => {
                let iterator = SingleGenerationChangedKeysIter::new(
                    &self.db,
                    state.get_to_generation_id(),
                    state.get_from_collection_key(),
                )?;

                state.diff_collection_records_sync(iterator, None)
            }
        }
    }
}
