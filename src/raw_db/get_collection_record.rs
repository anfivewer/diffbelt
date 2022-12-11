use crate::collection::util::record_key::{OwnedRecordKey, RecordKey};
use crate::common::{CollectionValue, IsByteArray};
use crate::raw_db::{RawDb, RawDbError};
use crate::util::bytes::decrement;
use rocksdb::{Direction, IteratorMode, ReadOptions};

pub struct GetCollectionRecordOptions<'a> {
    pub record_key: RecordKey<'a>,
}

impl RawDb {
    pub async fn get_collection_record(
        &self,
        options: GetCollectionRecordOptions<'_>,
    ) -> Result<Option<(OwnedRecordKey, CollectionValue)>, RawDbError> {
        let db = self.db.clone();
        let record_key = options.record_key.to_owned();

        tokio::task::spawn_blocking(move || {
            let mut lower_record_key = record_key.clone();
            let record_collection_key = lower_record_key.get_collection_key_bytes_mut();
            decrement(record_collection_key);

            let iterator_mode = IteratorMode::From(record_key.get_byte_array(), Direction::Reverse);
            let mut opts = ReadOptions::default();
            opts.set_iterate_lower_bound(lower_record_key.get_byte_array());

            let iterator = db.iterator_opt(iterator_mode, opts);

            let record_key = record_key.as_ref();
            let collection_key = record_key.get_key();
            let generation_id = record_key.get_generation_id();
            let phantom_id = record_key.get_phantom_id();

            for item in iterator {
                let (key, value) = item?;

                let item_record_key =
                    RecordKey::validate(&key).or(Err(RawDbError::InvalidRecordKey))?;

                if item_record_key.get_key() != collection_key {
                    break;
                }
                if item_record_key.get_phantom_id() != phantom_id {
                    continue;
                }

                if item_record_key.get_generation_id() <= generation_id {
                    let is_value_present = value.len() > 0;

                    return if is_value_present {
                        Ok(Some((
                            item_record_key.to_owned(),
                            CollectionValue::from_boxed_slice(value),
                        )))
                    } else {
                        Ok(None)
                    };
                }
            }

            Ok(None)
        })
        .await?
    }
}
