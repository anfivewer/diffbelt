use crate::collection::{Collection, GetReaderGenerationIdError};
use crate::common::OwnedGenerationId;
use crate::raw_db::{RawDb, RawDbError};
use std::collections::{HashMap, HashSet};

use crate::collection::methods::errors::CollectionMethodError;
use crate::database::constants::DATABASE_RAW_DB_CF;
use crate::messages::cursors::DatabaseCollectionCursorsTask;
use crate::messages::readers::{DatabaseCollectionReadersTask, GetReadersPointingToCollectionTask};
use crate::util::async_task_thread::AsyncTaskThread;
use std::sync::Arc;
use tokio::sync::{oneshot, watch, RwLock};

pub struct DatabaseInner {
    collections_for_deletion: Arc<RwLock<HashSet<String>>>,
    database_raw_db: Arc<RawDb>,
    collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
    readers: AsyncTaskThread<DatabaseCollectionReadersTask>,
    cursors: AsyncTaskThread<DatabaseCollectionCursorsTask>,
    stop_receiver: watch::Receiver<bool>,
}

pub enum GetReaderGenerationIdFnError {
    NoSuchCollection,
    NoSuchReader,
    RawDb(RawDbError),
}

impl DatabaseInner {
    pub fn new(
        collections_for_deletion: Arc<RwLock<HashSet<String>>>,
        database_raw_db: Arc<RawDb>,
        collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
        readers: AsyncTaskThread<DatabaseCollectionReadersTask>,
        cursors: AsyncTaskThread<DatabaseCollectionCursorsTask>,
        stop_receiver: watch::Receiver<bool>,
    ) -> Self {
        Self {
            collections_for_deletion,
            database_raw_db,
            collections,
            readers,
            cursors,
            stop_receiver,
        }
    }

    pub fn get_reader_generation_id_sync(
        &self,
        collection_name: &str,
        reader_name: &str,
    ) -> Result<Option<OwnedGenerationId>, GetReaderGenerationIdFnError> {
        let collections_lock = self.collections.blocking_read();

        let collection = collections_lock
            .get(collection_name)
            .ok_or(GetReaderGenerationIdFnError::NoSuchCollection)?;

        let collection = collection.clone();

        drop(collections_lock);

        collection
            .get_reader_generation_id(reader_name)
            .map_err(|err| match err {
                GetReaderGenerationIdError::NoSuchReader => {
                    GetReaderGenerationIdFnError::NoSuchReader
                }
                GetReaderGenerationIdError::RawDb(err) => GetReaderGenerationIdFnError::RawDb(err),
            })
    }

    fn mark_collection_for_deletion_sync(&self, collection_name: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_name.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_name);

        self.database_raw_db
            .put_cf_sync(DATABASE_RAW_DB_CF, key.as_bytes(), b"")?;

        Ok(())
    }

    fn unmark_collection_for_deletion_sync(&self, collection_name: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_name.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_name);

        self.database_raw_db
            .delete_cf_sync(DATABASE_RAW_DB_CF, key.as_bytes())?;

        Ok(())
    }

    pub fn is_marked_for_deletion_sync(&self, collection_name: &str) -> Result<bool, RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_name.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_name);

        let is_marked = self
            .database_raw_db
            .get_cf_sync(DATABASE_RAW_DB_CF, key.as_bytes())?
            .is_some();

        Ok(is_marked)
    }

    pub async fn start_delete_collection(&self, collection_name: &str) -> Result<(), RawDbError> {
        // Now we need remove this collection from `Database.collections` and remove its raw_db,
        // cleanup collection data from meta_raw_db of `Database`
        // Order here matters, we need expect that process can crash in any moment,
        // after restart it should work and collection should be in one of states:
        //   - didn't deleted
        //   - deleted
        //   - marked as deleted (and then deleted on `Database::open`)

        // Mark for deletion, this will delete this collection on database open
        self.mark_collection_for_deletion_sync(collection_name)?;

        // Block creation of this collection
        let mut collections_for_deletion = self.collections_for_deletion.write().await;
        collections_for_deletion.insert(collection_name.to_string());
        drop(collections_for_deletion);

        // Remove from collections to not hold Arc<Collection>
        let mut collections_lock = self.collections.write().await;
        collections_lock.remove(collection_name);
        drop(collections_lock);

        Ok(())
    }

    pub fn finish_delete_collection_sync(&self, collection_name: &str) -> Result<(), RawDbError> {
        self.unmark_collection_for_deletion_sync(collection_name)?;

        Ok(())
    }

    pub async fn remove_readers_pointing_to_collection(
        &self,
        collection_name: Arc<str>,
    ) -> Result<(), CollectionMethodError> {
        let (sender, receiver) = oneshot::channel();

        self.add_readers_task(
            DatabaseCollectionReadersTask::GetReadersPointingToCollectionExceptThisOne(
                GetReadersPointingToCollectionTask {
                    collection_name,
                    sender,
                },
            ),
        )
        .await;

        let readers = receiver.await.map_err(CollectionMethodError::OneshotRecv)?;

        let mut by_collection = HashMap::new();

        for reader in readers {
            let (_, reader_names) = by_collection
                .entry(reader.owner_collection_name)
                .or_insert_with(|| (Option::<Arc<Collection>>::None, Vec::new()));

            reader_names.push(reader.reader_name);
        }

        {
            let collections = self.collections.read().await;

            for (collection_name, collection_and_reader_names) in by_collection.iter_mut() {
                collection_and_reader_names.0 =
                    collections.get(collection_name.as_ref()).map(|x| x.clone());
            }
        }

        for (_, (collection, reader_names)) in by_collection {
            let Some(collection) = collection else {
                continue;
            };

            for reader_name in reader_names {
                collection.inner_remove_reader(reader_name).await?;
            }
        }

        Ok(())
    }

    pub async fn add_readers_task(&self, task: DatabaseCollectionReadersTask) {
        self.readers.add_task(task).await
    }

    pub async fn add_cursors_task(&self, task: DatabaseCollectionCursorsTask) {
        self.cursors.add_task(task).await
    }

    pub fn on_database_drop(&self) {
        self.readers.send_stop();
        self.cursors.send_stop();
    }

    pub fn stop_receiver(&self) -> watch::Receiver<bool> {
        self.stop_receiver.clone()
    }
}
