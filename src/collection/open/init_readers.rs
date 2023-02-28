use crate::collection::constants::COLLECTION_CF_META;

use crate::collection::open::CollectionOpenError;
use crate::collection::util::reader_value::ReaderValue;
use crate::collection::Collection;
use crate::messages::readers::{DatabaseCollecitonReadersTask, UpdateReaderTask};
use crate::util::tokio::spawn_blocking_async;
use std::str::from_utf8;
use std::sync::Arc;

pub async fn init_readers(collection: Arc<Collection>) -> Result<(), CollectionOpenError> {
    spawn_blocking_async(async move {
        let raw_db = &collection.raw_db;
        let database_inner = &collection.database_inner;

        let readers = raw_db
            .get_range_sync_cf(COLLECTION_CF_META, b"reader:", b"reader;")
            .map_err(CollectionOpenError::RawDb)?;

        for (key, value) in readers {
            let reader_name = &key[(b"reader:".len())..];
            let reader_name = from_utf8(reader_name).or(Err(CollectionOpenError::InvalidUtf8))?;
            let reader_value =
                ReaderValue::from_slice(&value).or(Err(CollectionOpenError::InvalidReaderValue))?;

            let generation_id = reader_value.get_generation_id();
            let to_collection_name = reader_value.get_collection_name();
            let to_collection_name = if to_collection_name.is_empty() {
                collection.name.clone()
            } else {
                Arc::from(to_collection_name)
            };

            database_inner
                .add_readers_task(DatabaseCollecitonReadersTask::UpdateReader(
                    UpdateReaderTask {
                        owner_collection_name: collection.name.clone(),
                        to_collection_name: Some(to_collection_name),
                        reader_name: Arc::from(reader_name),
                        generation_id: Arc::new(generation_id.to_owned()),
                    },
                ))
                .await;
        }

        Ok(())
    })
    .await
    .map_err(|_| CollectionOpenError::JoinError)?
}
