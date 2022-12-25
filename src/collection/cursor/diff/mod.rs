use crate::common::{GenerationId, KeyValueDiff, OwnedGenerationId};

use crate::collection::cursor::util::BaseCursor;

use crate::common::generation_id::GenerationIdSource;
use crate::raw_db::diff_collection_records::DiffCursorState;

pub mod get_pack;

pub struct DiffCursor {
    prev_cursor_id: Option<String>,
    from_generation_id: GenerationIdSource,
    to_generation_id: OwnedGenerationId,
    omit_intermediate_values: bool,
    raw_db_cursor_state: Option<DiffCursorState>,
}

pub struct DiffCursorNewOptions {
    pub from_generation_id: GenerationIdSource,
    // Result can be returned with generation_id <= to_generation_id_loose
    pub to_generation_id_loose: OwnedGenerationId,
    pub omit_intermediate_values: bool,
}

pub struct DiffCursorPack {
    pub from_generation_id: Option<OwnedGenerationId>,
    pub to_generation_id: OwnedGenerationId,
    pub items: Vec<KeyValueDiff>,
    pub next_cursor: Option<DiffCursor>,
}

impl DiffCursor {
    pub fn new(options: DiffCursorNewOptions) -> Self {
        let DiffCursorNewOptions {
            from_generation_id,
            to_generation_id_loose,
            omit_intermediate_values,
        } = options;

        Self {
            prev_cursor_id: None,
            from_generation_id,
            to_generation_id: to_generation_id_loose,
            omit_intermediate_values,
            raw_db_cursor_state: None,
        }
    }

    pub fn get_to_generation_id(&self) -> GenerationId<'_> {
        self.to_generation_id.as_ref()
    }
}

impl BaseCursor for DiffCursor {
    fn get_prev_cursor_id(&self) -> Option<&str> {
        self.prev_cursor_id.as_ref().map(|x| x.as_str())
    }
}
