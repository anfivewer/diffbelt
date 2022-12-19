use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::collection::util::reader_value::ReaderValue;
use crate::common::reader::ReaderRecord;

use crate::util::tokio::spawn_blocking_async;
use std::str::from_utf8;

pub struct ListReadersOk {
    pub items: Vec<ReaderRecord>,
}

impl Collection {
    pub async fn list_readers(&self) -> Result<ListReadersOk, CollectionMethodError> {
        if !self.is_manual {
            return Err(CollectionMethodError::UnsupportedOperationForThisCollectionType);
        }

        let meta_raw_db = self.meta_raw_db.clone();

        let result =
            spawn_blocking_async(async move { meta_raw_db.get_range_sync(b"reader:", b"reader;") })
                .await
                .or(Err(CollectionMethodError::TaskJoin))??;

        let mut items = Vec::<ReaderRecord>::with_capacity(result.len());

        for (key, value) in result {
            let reader_id = &key[(b"reader:".len())..];
            let reader_id = from_utf8(reader_id).or(Err(CollectionMethodError::InvalidUtf8))?;
            let reader_value = ReaderValue::from_slice(&value)
                .or(Err(CollectionMethodError::InvalidReaderValue))?;

            let generation_id = reader_value.get_generation_id();
            let collection_id = reader_value.get_collection_id();
            let collection_id = if collection_id.is_empty() {
                None
            } else {
                Some(collection_id.to_string())
            };

            items.push(ReaderRecord {
                reader_id: reader_id.to_string(),
                generation_id: generation_id.to_opt_owned_if_empty(),
                collection_id,
            });
        }

        Ok(ListReadersOk { items })
    }
}
