use crate::collection::constants::COLLECTION_CF_META;
use crate::common::{GenerationId, IsByteArray};
use crate::raw_db::update_reader::RawDbUpdateReaderOptions;
use crate::raw_db::{RawDb, RawDbError};
use rocksdb::WriteBatchWithTransaction;

pub struct RawDbUpdateReader<'a> {
    pub reader_name: &'a str,
    pub generation_id: GenerationId<'a>,
}

pub struct RawDbCommitGenerationOptions<'a> {
    pub generation_id: GenerationId<'a>,
    pub next_generation_id: GenerationId<'a>,
    pub update_readers: Option<Vec<RawDbUpdateReader<'a>>>,
}

impl RawDb {
    pub fn commit_generation_sync(
        &self,
        options: RawDbCommitGenerationOptions<'_>,
    ) -> Result<(), RawDbError> {
        let RawDbCommitGenerationOptions {
            generation_id,
            next_generation_id,
            update_readers,
        } = options;

        let mut batch = WriteBatchWithTransaction::<false>::default();

        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        batch.put_cf(&meta_cf, b"generation_id", generation_id.get_byte_array());
        batch.put_cf(
            &meta_cf,
            b"next_generation_id",
            next_generation_id.get_byte_array(),
        );

        if let Some(update_readers) = update_readers {
            for update in update_readers {
                let RawDbUpdateReader {
                    reader_name,
                    generation_id,
                } = update;

                self.update_reader_batch(
                    &mut batch,
                    meta_cf.clone(),
                    RawDbUpdateReaderOptions {
                        reader_name,
                        generation_id,
                    },
                )?;
            }
        }

        db.write(batch)?;

        Ok(())
    }
}
