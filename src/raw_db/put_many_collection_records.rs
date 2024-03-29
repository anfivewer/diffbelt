use crate::collection::util::generation_key::OwnedGenerationKey;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{IsByteArray, OwnedCollectionValue};
use crate::raw_db::{RawDb, RawDbError};

use crate::collection::constants::{COLLECTION_CF_GENERATIONS, COLLECTION_CF_GENERATIONS_SIZE};
use crate::raw_db::put_collection_record::unwrap_option_ref_or;
use crate::util::bytes::ONE_U32_BE;
use rocksdb::WriteBatchWithTransaction;

pub struct PutManyCollectionRecordsItem {
    pub record_key: OwnedRecordKey,
    pub value: Option<OwnedCollectionValue>,
}

pub struct PutManyCollectionRecordsOptions {
    pub items: Vec<PutManyCollectionRecordsItem>,
}

impl RawDb {
    pub async fn put_many_collection_records(
        &self,
        options: PutManyCollectionRecordsOptions,
    ) -> Result<(), RawDbError> {
        let db = self.db.clone();
        let items = options.items;

        tokio::task::spawn_blocking(move || {
            let db = db.get_db();

            let generations_cf = db
                .cf_handle(COLLECTION_CF_GENERATIONS)
                .ok_or(RawDbError::CfHandle)?;
            let generations_size_cf = db
                .cf_handle(COLLECTION_CF_GENERATIONS_SIZE)
                .ok_or(RawDbError::CfHandle)?;

            let mut batch = WriteBatchWithTransaction::<false>::default();

            for item in items {
                let record_key_ref = item.record_key.as_ref();
                let is_phantom = record_key_ref.get_phantom_id().get_byte_array().len() > 0;

                let value_bytes = unwrap_option_ref_or(&item.value, b"");
                batch.put(record_key_ref.get_byte_array(), value_bytes);

                if !is_phantom {
                    let generation_id = record_key_ref.get_generation_id();

                    let generation_key =
                        OwnedGenerationKey::new(generation_id, record_key_ref.get_collection_key())
                            .or(Err(RawDbError::InvalidGenerationKey))?;

                    batch.put_cf(&generations_cf, generation_key.get_byte_array(), b"");
                    batch.merge_cf(
                        &generations_size_cf,
                        generation_id.get_byte_array(),
                        ONE_U32_BE,
                    );
                }
            }

            let result = db.write(batch)?;

            Ok(result)
        })
        .await?
    }
}
