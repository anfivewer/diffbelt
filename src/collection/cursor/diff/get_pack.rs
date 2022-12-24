use crate::collection::cursor::diff::{DiffCursor, DiffCursorPack};
use crate::collection::methods::errors::CollectionMethodError;
use crate::raw_db::RawDb;
use std::sync::Arc;

pub struct GetPackOptions {
    pub this_cursor_id: Option<String>,
    pub db: Arc<RawDb>,
}

const PACK_LIMIT: usize = 200;

impl DiffCursor {
    pub fn get_pack_sync(
        &self,
        _options: GetPackOptions,
    ) -> Result<DiffCursorPack, CollectionMethodError> {
        if !self.omit_intermediate_values {
            // TODO: implement intermediate values in the diff
            return Err(CollectionMethodError::NotImplementedYet);
        }

        //

        todo!()
    }
}
