use crate::collection::cursor::query::{QueryCursor, QueryCursorPack};

pub struct GetPackOptions {
    pub this_cursor_id: Option<String>,
}

impl QueryCursor {
    pub fn get_pack_sync(&self, _options: GetPackOptions) -> QueryCursorPack {
        todo!()
    }
}
