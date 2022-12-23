use crate::common::{GenerationId, KeyValue, OwnedGenerationId, OwnedPhantomId};

pub mod get_pack;

pub struct QueryCursor {
    prev_cursor_id: Option<String>,
    generation_id: OwnedGenerationId,
    phantom_id: Option<OwnedPhantomId>,
}

pub struct QueryCursorPack {
    pub items: Vec<KeyValue>,
    pub next_cursor: Option<QueryCursor>,
}

impl QueryCursor {
    pub fn new(generation_id: OwnedGenerationId, phantom_id: Option<OwnedPhantomId>) -> Self {
        QueryCursor {
            prev_cursor_id: None,
            generation_id,
            phantom_id,
        }
    }

    pub fn get_generation_id(&self) -> GenerationId<'_> {
        self.generation_id.as_ref()
    }

    pub fn get_prev_cursor_id(&self) -> Option<&String> {
        self.prev_cursor_id.as_ref()
    }
}
