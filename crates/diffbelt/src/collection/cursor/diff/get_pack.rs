use crate::collection::cursor::diff::{DiffCursorPack, GenerationIdSource};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::common::reader::ReaderDef;
use crate::database::config::DatabaseConfig;
use crate::database::cursors::diff::DiffCursor;
use crate::database::{DatabaseInner, GetReaderGenerationIdFnError};
use crate::raw_db::diff_collection_records::{
    DiffCollectionRecordsOk, DiffCollectionRecordsOptions,
};
use std::sync::Arc;

pub struct GetPackOptions {
    pub db: CollectionRawDb,
    pub db_inner: Arc<DatabaseInner>,
    pub config: Arc<DatabaseConfig>,
}

impl DiffCursor {
    pub fn get_pack_sync(
        &self,
        options: GetPackOptions,
    ) -> Result<DiffCursorPack, CollectionMethodError> {
        if !self.omit_intermediate_values {
            // TODO: implement intermediate values in the diff
            return Err(CollectionMethodError::NotImplementedYet);
        }

        let GetPackOptions {
            db,
            db_inner,
            config,
        } = options;

        let from_generation_id = match &self.from_generation_id {
            GenerationIdSource::Value(value) => value.clone(),
            GenerationIdSource::Reader(ReaderDef {
                collection_name,
                reader_name,
            }) => match collection_name {
                Some(collection_name) => db_inner
                    .get_reader_generation_id_sync(&collection_name, &reader_name)
                    .or_else(|err| {
                        let err = match err {
                            GetReaderGenerationIdFnError::NoSuchReader => {
                                CollectionMethodError::NoSuchReader
                            }
                            GetReaderGenerationIdFnError::NoSuchCollection => {
                                CollectionMethodError::NoSuchCollection
                            }
                            GetReaderGenerationIdFnError::RawDb(err) => {
                                CollectionMethodError::RawDb(err)
                            }
                        };
                        Err(err)
                    })?,
                None => {
                    let reader = db.get_reader_sync(&reader_name)?;
                    reader.generation_id
                }
            },
        };

        let result = db.diff_collection_records_sync(DiffCollectionRecordsOptions {
            from_generation_id: from_generation_id.as_ref().map(|id| id.as_ref()),
            to_generation_id_loose: self.to_generation_id.as_ref(),
            prev_diff_state: self.raw_db_cursor_state.as_ref(),
            limit: config.diff_pack_limit,
            records_to_view_limit: config.diff_pack_records_limit,
            total_count_in_generations_limit: config.diff_changes_limit,
        })?;

        let DiffCollectionRecordsOk {
            to_generation_id,
            items,
            next_diff_state,
        } = result;

        Ok(DiffCursorPack {
            from_generation_id,
            to_generation_id,
            items,
            next_diff_state,
        })
    }
}
