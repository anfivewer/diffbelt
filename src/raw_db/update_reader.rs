use crate::collection::util::reader_value::{OwnedReaderValue, ReaderValue};

use crate::common::{GenerationId, IsByteArray};
use crate::raw_db::{RawDb, RawDbError};

pub struct RawDbCreateReaderOptions<'a> {
    pub reader_id: &'a str,
    pub collection_id: Option<&'a str>,
}

pub struct RawDbUpdateReaderOptions<'a> {
    pub reader_id: &'a str,
    pub generation_id: Option<GenerationId<'a>>,
}

pub struct RawDbDeleteReaderOptions<'a> {
    pub reader_id: &'a str,
}

pub enum RawDbCreateReaderResult {
    Created,
    AlreadyExists(OwnedReaderValue),
}

impl RawDb {
    pub fn create_reader_sync(
        &self,
        options: RawDbCreateReaderOptions<'_>,
    ) -> Result<RawDbCreateReaderResult, RawDbError> {
        let meta_cf = self.db.cf_handle("meta").ok_or(RawDbError::CfHandle)?;

        let mut key = String::with_capacity("reader:".len() + options.reader_id.len());
        key.push_str("reader:");
        key.push_str(options.reader_id);

        let expected_value = OwnedReaderValue::new(options.collection_id, None)
            .or(Err(RawDbError::InvalidReaderValue))?;
        let expected_value = expected_value.get_byte_array();

        self.db.merge_cf(&meta_cf, &key, expected_value)?;

        let result = self.db.get_cf(&meta_cf, &key)?;

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
        let meta_cf = self.db.cf_handle("meta").ok_or(RawDbError::CfHandle)?;

        let mut key = String::with_capacity("reader:".len() + options.reader_id.len());
        key.push_str("reader:");
        key.push_str(options.reader_id);

        let value = self.db.get_cf(&meta_cf, &key)?;

        let value = match value {
            Some(value) => value,
            None => {
                return Err(RawDbError::NoSuchReader);
            }
        };

        let old_value = ReaderValue::from_slice(&value).or(Err(RawDbError::InvalidReaderValue))?;
        let collection_id = old_value.get_collection_id();
        let generation_id = options.generation_id;

        let new_value = OwnedReaderValue::new(Some(collection_id), generation_id)
            .or(Err(RawDbError::InvalidReaderValue))?;

        self.db.put_cf(&meta_cf, &key, new_value.get_byte_array())?;

        Ok(())
    }

    pub fn delete_reader_sync(
        &self,
        options: RawDbDeleteReaderOptions<'_>,
    ) -> Result<(), RawDbError> {
        let meta_cf = self.db.cf_handle("meta").ok_or(RawDbError::CfHandle)?;

        let mut key = String::with_capacity("reader:".len() + options.reader_id.len());
        key.push_str("reader:");
        key.push_str(options.reader_id);

        self.db.delete_cf(&meta_cf, &key)?;

        Ok(())
    }
}
