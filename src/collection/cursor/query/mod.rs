use crate::common::{GenerationId, KeyValue, OwnedGenerationId, OwnedPhantomId};

use crate::collection::cursor::util::BaseCursor;
use crate::raw_db::query_collection_records::LastAndNextRecordKey;

pub mod get_pack;

pub struct QueryCursor {
    prev_cursor_id: Option<String>,
    generation_id: OwnedGenerationId,
    phantom_id: Option<OwnedPhantomId>,
    last_and_next_record_key: Option<LastAndNextRecordKey>,
}

pub struct QueryCursorNewOptions {
    pub generation_id: OwnedGenerationId,
    pub phantom_id: Option<OwnedPhantomId>,
}

pub struct QueryCursorPack {
    pub items: Vec<KeyValue>,
    pub next_cursor: Option<QueryCursor>,
}

impl QueryCursor {
    pub fn new(options: QueryCursorNewOptions) -> Self {
        QueryCursor {
            prev_cursor_id: None,
            generation_id: options.generation_id,
            phantom_id: options.phantom_id,
            last_and_next_record_key: None,
        }
    }

    pub fn get_generation_id(&self) -> GenerationId<'_> {
        self.generation_id.as_ref()
    }
}

impl BaseCursor for QueryCursor {
    fn get_prev_cursor_id(&self) -> Option<&str> {
        self.prev_cursor_id.as_ref().map(|x| x.as_str())
    }
}
