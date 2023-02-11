use crate::collection::util::record_key::{OwnedParsedRecordKey, RecordKey};
use crate::common::IsByteArray;
use crate::raw_db::RawDbError;

pub struct LastRecord {
    pub key: OwnedParsedRecordKey,
    pub value: Option<Box<[u8]>>,
}

pub type RocksDbIteratorItem = Result<(Box<[u8]>, Box<[u8]>), rocksdb::Error>;

pub fn get_initial_last_record(
    db: &rocksdb::DB,
    mut iterator: impl Iterator<Item = RocksDbIteratorItem>,
    last_record_key: Option<RecordKey<'_>>,
    is_from_record_key_specified: bool,
) -> Result<Option<LastRecord>, RawDbError> {
    match last_record_key {
        Some(key) => {
            let result = db.get(key.get_byte_array())?;
            let value = match result {
                Some(x) => x,
                None => {
                    return Err(RawDbError::CursorDidNotFoundRecord);
                }
            };

            return Ok(Some(LastRecord {
                key: key.to_owned_parsed(),
                value: Some(value.into()),
            }));
        }
        None => {}
    }

    let first_kv = iterator.by_ref().next();

    match first_kv {
        Some(kv) => {
            let (key, value) = kv?;
            let record_key = OwnedParsedRecordKey::from_boxed_slice(key)
                .or(Err(RawDbError::InvalidRecordKey))?;
            Ok(Some(LastRecord {
                key: record_key,
                value: Some(value),
            }))
        }
        None => {
            if is_from_record_key_specified {
                // If record was specified, it should be present in the collection,
                // because existance of cursor should block garbage collection
                Err(RawDbError::CursorDidNotFoundRecord)
            } else {
                Ok(None)
            }
        }
    }
}
