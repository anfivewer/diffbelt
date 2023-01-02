use crate::collection::{Collection, GetReaderGenerationIdError};
use crate::common::OwnedGenerationId;
use crate::raw_db::{RawDb, RawDbError};
use std::collections::{HashMap, HashSet};

use crate::database::constants::DATABASE_RAW_DB_CF;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DatabaseInner {
    collections_for_deletion: Arc<RwLock<HashSet<String>>>,
    database_raw_db: Arc<RawDb>,
    collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
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
    ) -> Self {
        Self {
            collections_for_deletion,
            database_raw_db,
            collections,
        }
    }

    pub fn get_reader_generation_id_sync(
        &self,
        collection_id: &str,
        reader_id: &str,
    ) -> Result<Option<OwnedGenerationId>, GetReaderGenerationIdFnError> {
        let collections_lock = self.collections.blocking_read();

        let collection = collections_lock
            .get(collection_id)
            .ok_or(GetReaderGenerationIdFnError::NoSuchCollection)?;

        let collection = collection.clone();

        drop(collections_lock);

        collection
            .get_reader_generation_id(reader_id)
            .map_err(|err| match err {
                GetReaderGenerationIdError::NoSuchReader => {
                    GetReaderGenerationIdFnError::NoSuchReader
                }
                GetReaderGenerationIdError::RawDb(err) => GetReaderGenerationIdFnError::RawDb(err),
            })
    }

    fn mark_collection_for_deletion_sync(&self, collection_id: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        self.database_raw_db
            .put_cf_sync(DATABASE_RAW_DB_CF, key.as_bytes(), b"")?;

        Ok(())
    }

    fn unmark_collection_for_deletion_sync(&self, collection_id: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        self.database_raw_db
            .delete_cf_sync(DATABASE_RAW_DB_CF, key.as_bytes())?;

        Ok(())
    }

    async fn unmark_collection_for_deletion(&self, collection_id: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        self.database_raw_db
            .delete_cf(DATABASE_RAW_DB_CF, key.into_bytes().into_boxed_slice())
            .await?;

        Ok(())
    }

    pub fn is_marked_for_deletion_sync(&self, collection_id: &str) -> Result<bool, RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        let is_marked = self
            .database_raw_db
            .get_cf_sync(DATABASE_RAW_DB_CF, key.as_bytes())?
            .is_some();

        Ok(is_marked)
    }

    pub async fn start_delete_collection(&self, collection_id: &str) -> Result<(), RawDbError> {
        // Now we need remove this collection from `Database.collections` and remove its raw_db,
        // cleanup collection data from meta_raw_db of `Database`
        // Order here matters, we need expect that process can crash in any moment,
        // after restart it should work and collection should be in one of states:
        //   - didn't deleted
        //   - deleted
        //   - marked as deleted (and then deleted on `Database::open`)

        // Mark for deletion, this will delete this collection on database open
        self.mark_collection_for_deletion_sync(collection_id)?;

        // Block creation of this collection
        let mut collections_for_deletion = self.collections_for_deletion.write().await;
        collections_for_deletion.insert(collection_id.to_string());
        drop(collections_for_deletion);

        // Remove from collections to not hold Arc<Collection>
        let mut collections_lock = self.collections.write().await;
        collections_lock.remove(collection_id);
        drop(collections_lock);

        Ok(())
    }

    pub fn finish_delete_collection_sync(&self, collection_id: &str) -> Result<(), RawDbError> {
        self.unmark_collection_for_deletion_sync(collection_id)?;

        Ok(())
    }
}
