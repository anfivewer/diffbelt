use crate::collection::constants::{COLLECTION_CF_GENERATIONS, COLLECTION_CF_GENERATIONS_SIZE};
use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{CollectionKey, GenerationId, IsByteArray, PhantomId};
use crate::raw_db::{RawDb, RawDbError};

use rocksdb::{Direction, IteratorMode, ReadOptions, WriteBatchWithTransaction};

pub struct RemoveAllRecordsOfGenerationSyncOptions<'a> {
    pub generation_id: GenerationId<'a>,
}

impl RawDb {
    pub fn remove_all_records_of_generation_sync(
        &self,
        options: RemoveAllRecordsOfGenerationSyncOptions<'_>,
    ) -> Result<(), RawDbError> {
        let generation_id = options.generation_id;

        let mut batch = WriteBatchWithTransaction::<false>::default();

        let db = self.db.get_db();

        let generations_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS)
            .ok_or(RawDbError::CfHandle)?;
        let generations_size_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS_SIZE)
            .ok_or(RawDbError::CfHandle)?;

        let upper_generation_id = generation_id.incremented();

        let upper_generation_key =
            OwnedGenerationKey::new(upper_generation_id.as_ref(), CollectionKey::empty())
                .or(Err(RawDbError::InvalidGenerationKey))?;

        let generation_key = OwnedGenerationKey::new(generation_id, CollectionKey::empty())
            .or(Err(RawDbError::InvalidGenerationKey))?;

        let iterator_mode = IteratorMode::From(generation_key.get_byte_array(), Direction::Forward);
        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(upper_generation_key.get_byte_array());

        let db = self.db.get_db();

        let iterator = db.iterator_cf_opt(&generations_cf, opts, iterator_mode);

        for item in iterator {
            let (key, _) = item?;
            let item_generation_key =
                GenerationKey::validate(&key).or(Err(RawDbError::InvalidGenerationKey))?;

            if item_generation_key.get_generation_id() != generation_id {
                break;
            }

            let collection_key = item_generation_key.get_collection_key();

            batch.delete_cf(&generations_cf, &key);
            batch.delete_cf(
                &generations_size_cf,
                item_generation_key.get_generation_id().get_byte_array(),
            );

            let record_key = OwnedRecordKey::new(collection_key, generation_id, PhantomId::empty())
                .or(Err(RawDbError::InvalidRecordKey))?;

            batch.delete(record_key.get_byte_array());
        }

        db.write(batch)?;

        Ok(())
    }
}
