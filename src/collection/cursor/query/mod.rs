use crate::common::{GenerationId, KeyValue, OwnedGenerationId, OwnedPhantomId};

use crate::database::cursors::query::{QueryCursor, QueryCursorPublicId};
use crate::raw_db::query_collection_records::LastAndNextRecordKey;

pub mod get_pack;

pub struct QueryCursorNewOptions {
    pub generation_id: OwnedGenerationId,
    pub phantom_id: Option<OwnedPhantomId>,
}

pub struct QueryCursorPack {
    pub items: Vec<KeyValue>,
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

impl QueryCursor {
    pub fn new(options: QueryCursorNewOptions) -> Self {
        QueryCursor {
            public_id: QueryCursorPublicId(0),
            generation_id: options.generation_id,
            phantom_id: options.phantom_id,
            last_and_next_record_key: None,
        }
    }

    pub fn get_generation_id(&self) -> GenerationId<'_> {
        self.generation_id.as_ref()
    }
}
