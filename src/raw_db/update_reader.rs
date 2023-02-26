use crate::collection::constants::COLLECTION_CF_META;
use crate::collection::util::reader_value::{OwnedReaderValue, ReaderValue};
use rocksdb::{BoundColumnFamily, WriteBatchWithTransaction};
use std::sync::Arc;

use crate::common::reader::ReaderState;
use crate::common::{GenerationId, IsByteArray};
use crate::raw_db::{RawDb, RawDbError};

pub struct RawDbCreateReaderOptions<'a> {
    pub reader_name: &'a str,
    pub collection_name: Option<&'a str>,
    pub generation_id: Option<GenerationId<'a>>,
}

pub struct RawDbUpdateReaderOptions<'a> {
    pub reader_name: &'a str,
    pub generation_id: Option<GenerationId<'a>>,
}

pub struct RawDbDeleteReaderOptions<'a> {
    pub reader_name: &'a str,
}

pub enum RawDbCreateReaderResult {
    Created,
    AlreadyExists(OwnedReaderValue),
}

impl RawDb {
    pub fn get_reader_sync(&self, reader_name: &str) -> Result<ReaderState, RawDbError> {
        let db = self.db.get_db();

        let meta_cf = db.cf_handle("meta").ok_or(RawDbError::CfHandle)?;

        let mut key = String::with_capacity("reader:".len() + reader_name.len());
        key.push_str("reader:");
        key.push_str(reader_name);

        let result = db.get_cf(&meta_cf, key)?;

        match result {
            Some(value) => {
                let reader_value =
                    OwnedReaderValue::from_vec(value).or(Err(RawDbError::InvalidReaderValue))?;

                let reader_value = reader_value.as_ref();

                // TODO: parse method
                let collection_name = {
                    let collection_name = reader_value.get_collection_name();
                    if collection_name.is_empty() {
                        None
                    } else {
                        Some(collection_name.to_string())
                    }
                };
                let generation_id = reader_value.get_generation_id().to_opt_owned_if_empty();

                Ok(ReaderState {
                    collection_name,
                    generation_id,
                })
            }
            None => Err(RawDbError::NoSuchReader),
        }
    }

    pub fn create_reader_sync(
        &self,
        options: RawDbCreateReaderOptions<'_>,
    ) -> Result<RawDbCreateReaderResult, RawDbError> {
        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let RawDbCreateReaderOptions {
            reader_name,
            collection_name,
            generation_id,
        } = options;

        let mut key = String::with_capacity("reader:".len() + reader_name.len());
        key.push_str("reader:");
        key.push_str(reader_name);

        let expected_value = OwnedReaderValue::new(collection_name, generation_id)
            .or(Err(RawDbError::InvalidReaderValue))?;
        let expected_value = expected_value.get_byte_array();

        db.merge_cf(&meta_cf, &key, expected_value)?;

        let result = db.get_cf(&meta_cf, &key)?;

        match result {
            Some(value) => {
                let item_value =
                    OwnedReaderValue::from_vec(value).or(Err(RawDbError::InvalidReaderValue))?;
                let value = item_value.get_byte_array();

                if value == expected_value {
                    Ok(RawDbCreateReaderResult::Created)
                } else {
                    Ok(RawDbCreateReaderResult::AlreadyExists(item_value))
                }
            }
            None => Err(RawDbError::UpdateReader),
        }
    }

    pub fn update_reader_sync(
        &self,
        options: RawDbUpdateReaderOptions<'_>,
    ) -> Result<(), RawDbError> {
        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let mut key = String::with_capacity("reader:".len() + options.reader_name.len());
        key.push_str("reader:");
        key.push_str(options.reader_name);

        let value = db.get_cf(&meta_cf, &key)?;

        let value = match value {
            Some(value) => value,
            None => {
                return Err(RawDbError::NoSuchReader);
            }
        };

        let old_value = ReaderValue::from_slice(&value).or(Err(RawDbError::InvalidReaderValue))?;
        let collection_name = old_value.get_collection_name();
        let generation_id = options.generation_id;

        let new_value = OwnedReaderValue::new(Some(collection_name), generation_id)
            .or(Err(RawDbError::InvalidReaderValue))?;

        db.put_cf(&meta_cf, &key, new_value.get_byte_array())?;

        Ok(())
    }

    pub fn update_reader_batch(
        &self,
        batch: &mut WriteBatchWithTransaction<false>,
        meta_cf: Arc<BoundColumnFamily>,
        options: RawDbUpdateReaderOptions<'_>,
    ) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("reader:".len() + options.reader_name.len());
        key.push_str("reader:");
        key.push_str(options.reader_name);

        let value = self.db.get_db().get_cf(&meta_cf, &key)?;

        let value = match value {
            Some(value) => value,
            None => {
                return Err(RawDbError::NoSuchReader);
            }
        };

        let old_value = ReaderValue::from_slice(&value).or(Err(RawDbError::InvalidReaderValue))?;
        let collection_name = old_value.get_collection_name();
        let generation_id = options.generation_id;

        let new_value = OwnedReaderValue::new(Some(collection_name), generation_id)
            .or(Err(RawDbError::InvalidReaderValue))?;

        batch.put_cf(&meta_cf, &key, new_value.get_byte_array());

        Ok(())
    }

    pub fn delete_reader_sync(
        &self,
        options: RawDbDeleteReaderOptions<'_>,
    ) -> Result<(), RawDbError> {
        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let mut key = String::with_capacity("reader:".len() + options.reader_name.len());
        key.push_str("reader:");
        key.push_str(options.reader_name);

        db.delete_cf(&meta_cf, &key)?;

        Ok(())
    }
}
