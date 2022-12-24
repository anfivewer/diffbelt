use crate::common::{GenerationId, KeyValueDiff, OwnedGenerationId};

use crate::raw_db::diff_collection_records::DiffCursorState;

pub mod get_pack;

pub struct DiffCursor {
    prev_cursor_id: Option<String>,
    from_generation_id: OwnedGenerationId,
    to_generation_id: OwnedGenerationId,
    omit_intermediate_values: bool,
    raw_db_cursor_state: Option<DiffCursorState>,
}

pub struct ReaderDef {
    pub collection_id: String,
    pub reader_id: String,
}

pub enum GenerationIdSource {
    Value(OwnedGenerationId),
    Reader(ReaderDef),
}

pub struct DiffCursorNewOptions {
    pub from_generation_id: GenerationIdSource,
    // Result can be returned with generation_id <= to_generation_id_loose
    pub to_generation_id_loose: OwnedGenerationId,
    pub omit_intermediate_values: bool,
}

pub struct DiffCursorPack {
    pub items: Vec<KeyValueDiff>,
    pub next_cursor: Option<DiffCursor>,
}

impl DiffCursor {
    pub fn new(_options: DiffCursorNewOptions) -> Self {
        todo!()
    }

    pub fn get_to_generation_id(&self) -> GenerationId<'_> {
        self.to_generation_id.as_ref()
    }

    pub fn get_prev_cursor_id(&self) -> Option<&String> {
        self.prev_cursor_id.as_ref()
    }
}
