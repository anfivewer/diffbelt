use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::collection::util::reader_value::ReaderValue;
use crate::common::reader::ReaderRecord;

use crate::collection::constants::COLLECTION_CF_META;
use crate::util::tokio::spawn_blocking_async;
use std::str::from_utf8;

pub struct ListReadersOk {
    pub items: Vec<ReaderRecord>,
}

impl Collection {
    pub async fn list_readers(&self) -> Result<ListReadersOk, CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = spawn_blocking_async(async move {
            raw_db.get_range_sync_cf(COLLECTION_CF_META, b"reader:", b"reader;")
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        let mut items = Vec::<ReaderRecord>::with_capacity(result.len());

        for (key, value) in result {
            let reader_name = &key[(b"reader:".len())..];
            let reader_name = from_utf8(reader_name).or(Err(CollectionMethodError::InvalidUtf8))?;
            let reader_value = ReaderValue::from_slice(&value)
                .or(Err(CollectionMethodError::InvalidReaderValue))?;

            let generation_id = reader_value.get_generation_id();
            let collection_name = reader_value.get_collection_name();
            let collection_name = if collection_name.is_empty() {
                None
            } else {
                Some(collection_name.to_string())
            };

            items.push(ReaderRecord {
                reader_name: reader_name.to_string(),
                generation_id: generation_id.to_opt_owned_if_empty(),
                collection_name,
            });
        }

        Ok(ListReadersOk { items })
    }
}
