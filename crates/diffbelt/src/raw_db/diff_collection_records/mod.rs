use crate::collection::constants::{COLLECTION_CF_META, COLLECTION_META_GC_PHANTOM_ID_KEY};
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{GenerationId, KeyValueDiff, OwnedCollectionKey, OwnedGenerationId, PhantomId};
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
        // TODO!

        match mode {
            DiffStateMode::InMemory(in_memory) => {
                let capacity_hint = Some(in_memory.changed_keys.len());
                let iterator = InMemoryChangedKeysIter::new(in_memory.changed_keys);

                state.diff_collection_records_sync(
                    iterator,
                    capacity_hint,
                    &mut keys_to_remove,
                    gc_phantom_id,
                )
            }
            DiffStateMode::SingleGeneration => {
                let iterator = SingleGenerationChangedKeysIter::new(
                    db,
                    state.get_to_generation_id(),
                    state.get_from_collection_key(),
                )?;

                state.diff_collection_records_sync(
                    iterator,
                    None,
                    &mut keys_to_remove,
                    gc_phantom_id,
                )
            }
        }
    }
}
