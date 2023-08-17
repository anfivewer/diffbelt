use crate::common::{KeyValueDiff, OwnedGenerationId};

use crate::common::generation_id::GenerationIdSource;
use crate::database::cursors::diff::DiffCursor;
use crate::database::cursors::storage::CursorPublicId;
use crate::raw_db::diff_collection_records::DiffCursorState;

pub mod get_pack;

pub struct DiffCursorNewOptions {
    pub from_generation_id: GenerationIdSource,
    pub to_generation_id_loose: OwnedGenerationId,
    pub omit_intermediate_values: bool,
}

pub struct DiffCursorPack {
    pub from_generation_id: Option<OwnedGenerationId>,
    pub to_generation_id: OwnedGenerationId,
    pub items: Vec<KeyValueDiff>,
    pub next_diff_state: Option<DiffCursorState>,
}

impl DiffCursor {
    pub fn new(options: DiffCursorNewOptions) -> Self {
        let DiffCursorNewOptions {
            from_generation_id,
            to_generation_id_loose,
            omit_intermediate_values,
        } = options;

        DiffCursor {
            public_id: CursorPublicId(0),
            from_generation_id,
            to_generation_id: to_generation_id_loose,
            omit_intermediate_values,
            raw_db_cursor_state: None,
        }
    }
}
