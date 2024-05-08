use rocksdb::WriteBatchWithTransaction;

use crate::collection::constants::{
    COLLECTION_CF_GENERATIONS, COLLECTION_CF_GENERATIONS_SIZE, COLLECTION_CF_META,
};
use crate::collection::util::generation_key::OwnedGenerationKey;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{IsByteArray, OwnedCollectionValue, OwnedPhantomId};
use crate::raw_db::put_collection_record::unwrap_option_ref_or;
use crate::raw_db::{RawDb, RawDbError};
use crate::util::bytes::ONE_U32_BE;

pub struct PutManyCollectionRecordsItem {
    pub record_key: OwnedRecordKey,
    pub value: Option<OwnedCollectionValue>,
}

pub struct PutManyCollectionRecordsOptions {
    pub items: Vec<PutManyCollectionRecordsItem>,
    /// All `items` record_key MUST be with this phantom id
    pub phantom_id: Option<OwnedPhantomId>,
}

impl RawDb {
    pub async fn put_many_collection_records(
        &self,
        options: PutManyCollectionRecordsOptions,
    ) -> Result<(), RawDbError> {
        let PutManyCollectionRecordsOptions { items, phantom_id } = options;

        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let db = db.get_db();

            let meta_cf = db
                .cf_handle(COLLECTION_CF_META)
                .ok_or(RawDbError::CfHandle)?;

            let is_phantom_exists =
                Self::is_phantom_exists(db, &meta_cf, phantom_id.as_ref().map(|x| x.as_ref()))?;
            if !is_phantom_exists {
                return Err(RawDbError::NoSuchPhantom);
            }

            let generations_cf = db
                .cf_handle(COLLECTION_CF_GENERATIONS)
                .ok_or(RawDbError::CfHandle)?;
            let generations_size_cf = db
                .cf_handle(COLLECTION_CF_GENERATIONS_SIZE)
                .ok_or(RawDbError::CfHandle)?;

            let mut batch = WriteBatchWithTransaction::<false>::default();

            for item in items {
                let record_key_ref = item.record_key.as_ref();
                let is_phantom = record_key_ref.get_phantom_id().is_some();

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
