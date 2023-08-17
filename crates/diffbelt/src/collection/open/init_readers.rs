use crate::collection::constants::COLLECTION_CF_META;

use crate::collection::open::CollectionOpenError;
use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::collection::util::reader_value::ReaderValue;
use crate::common::collection::CollectionName;
use crate::database::DatabaseInner;
use crate::messages::readers::{
    DatabaseCollectionReadersTask, UpdateReaderTask, UpdateReadersTask,
};
use crate::util::async_sync_call::async_sync_call;
use crate::util::tokio::spawn_blocking_async;
use std::str::from_utf8;
use std::sync::Arc;

pub async fn init_readers(
    collection_name: CollectionName,
    raw_db: CollectionRawDb,
    database_inner: Arc<DatabaseInner>,
) -> Result<(), CollectionOpenError> {
    spawn_blocking_async(async move {
        let readers = raw_db
            .get_range_sync_cf(COLLECTION_CF_META, b"reader:", b"reader;")
            .map_err(CollectionOpenError::RawDb)?;

        let mut updates = Vec::with_capacity(readers.len());

        for (key, value) in readers {
            let reader_name = &key[(b"reader:".len())..];
            let reader_name = from_utf8(reader_name).or(Err(CollectionOpenError::InvalidUtf8))?;
            let reader_value =
                ReaderValue::from_slice(&value).or(Err(CollectionOpenError::InvalidReaderValue))?;

            let generation_id = reader_value.get_generation_id();
            let to_collection_name = reader_value.get_collection_name();
            let to_collection_name = if to_collection_name.is_empty() {
                collection_name.clone()
            } else {
                Arc::from(to_collection_name)
            };

            updates.push(UpdateReaderTask {
                owner_collection_name: collection_name.clone(),
                to_collection_name: Some(to_collection_name),
                reader_name: Arc::from(reader_name),
                generation_id: generation_id.to_owned(),
                sender: None,
            });
        }

        let _: () = async_sync_call(|sender| {
            database_inner.add_readers_task(DatabaseCollectionReadersTask::UpdateReaders(
                UpdateReadersTask { updates, sender },
            ))
        })
        .await
        .map_err(CollectionOpenError::OneshotRecv)?;

        Ok(())
    })
    .await
    .map_err(|_| CollectionOpenError::JoinError)?
}
