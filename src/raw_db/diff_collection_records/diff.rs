use crate::raw_db::diff_collection_records::{
    DiffCollectionRecordsOptions, DiffCollectionRecordsResult,
};
use crate::raw_db::{RawDb, RawDbError};

impl RawDb {
    pub fn diff_collection_records_sync(
        &self,
        options: DiffCollectionRecordsOptions,
    ) -> Result<DiffCollectionRecordsResult, RawDbError> {
        let DiffCollectionRecordsOptions {
            from_generation_id: _,
            to_generation_id_loose: _,
            prev_diff_state: _,
            limit: _,
        } = options;

        todo!()
    }
}
