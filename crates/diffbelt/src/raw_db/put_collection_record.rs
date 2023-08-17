use crate::collection::util::generation_key::OwnedGenerationKey;
use crate::collection::util::record_key::RecordKey;
use crate::common::{CollectionValue, IsByteArray, OwnedCollectionValue};
use crate::raw_db::{RawDb, RawDbError};

use crate::collection::constants::{COLLECTION_CF_GENERATIONS, COLLECTION_CF_GENERATIONS_SIZE};
use crate::util::bytes::ONE_U32_BE;
use rocksdb::WriteBatchWithTransaction;

pub struct PutCollectionRecordOptions<'a> {
    pub record_key: RecordKey<'a>,
    pub value: Option<CollectionValue<'a>>,
}

impl RawDb {
    pub async fn put_collection_record(
        &self,
        options: PutCollectionRecordOptions<'_>,
    ) -> Result<(), RawDbError> {
        let db = self.db.clone();
        let record_key = options.record_key.to_owned();
        let value: Option<OwnedCollectionValue> = options.value.map(|x| x.to_owned());

        tokio::task::spawn_blocking(move || {
            let db = db.get_db();

            let generations_cf = db
                .cf_handle(COLLECTION_CF_GENERATIONS)
                .ok_or(RawDbError::CfHandle)?;
            let generations_size_cf = db
                .cf_handle(COLLECTION_CF_GENERATIONS_SIZE)
                .ok_or(RawDbError::CfHandle)?;

            let record_key_ref = record_key.as_ref();
            let is_phantom = record_key_ref.get_phantom_id().get_byte_array().len() > 0;

            let mut batch = WriteBatchWithTransaction::<false>::default();

            let value_bytes = unwrap_option_ref_or(&value, b"");
            batch.put(record_key.get_byte_array(), value_bytes);

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

            let result = db.write(batch)?;

            Ok(result)
        })
        .await?
    }
}

pub fn unwrap_option_ref_or<'a>(
    opt: &'a Option<OwnedCollectionValue>,
    default: &'a [u8],
) -> &'a [u8] {
    match opt {
        Some(value) => value.get_byte_array(),
        None => default,
    }
}
