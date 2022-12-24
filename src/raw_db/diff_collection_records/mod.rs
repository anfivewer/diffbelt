use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{GenerationId, KeyValueDiff};

pub mod diff;

pub struct DiffCollectionRecordsOptions<'a> {
    pub from_generation_id: GenerationId<'a>,
    // Not loose if `prev_diff_state` is specified
    pub to_generation_id_loose: GenerationId<'a>,
    pub prev_diff_state: Option<DiffCursorState>,
    pub limit: usize,
}

pub struct DiffCollectionRecordsResult {
    pub items: Vec<KeyValueDiff>,
    pub next_diff_state: Option<DiffCursorState>,
}

pub struct DiffCursorState {
    last_record_key: OwnedRecordKey,
    next_record_key: OwnedRecordKey,
}
