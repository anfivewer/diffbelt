use crate::collection::{Collection, GetReaderGenerationIdError};
use crate::common::OwnedGenerationId;
use crate::raw_db::{RawDb, RawDbError};
use std::collections::HashMap;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DatabaseInner {
    meta_raw_db: Arc<RawDb>,
    collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
}

pub enum GetReaderGenerationIdFnError {
    NoSuchCollection,
    NoSuchReader,
    RawDb(RawDbError),
}

impl DatabaseInner {
    pub fn new(
        meta_raw_db: Arc<RawDb>,
        collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
    ) -> Self {
        Self {
            meta_raw_db,
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

        self.meta_raw_db.put_sync(key.as_bytes(), b"")?;

        Ok(())
    }

    fn unmark_collection_for_deletion_sync(&self, collection_id: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        self.meta_raw_db.delete_sync(key.as_bytes())?;

        Ok(())
    }

    async fn unmark_collection_for_deletion(&self, collection_id: &str) -> Result<(), RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        self.meta_raw_db
            .delete(key.into_bytes().into_boxed_slice())
            .await?;

        Ok(())
    }

    pub fn is_marked_for_deletion_sync(&self, collection_id: &str) -> Result<bool, RawDbError> {
        let mut key = String::with_capacity("deleteCollection:".len() + collection_id.len());
        key.push_str("deleteCollection:");
        key.push_str(collection_id);

        let is_marked = self.meta_raw_db.get_sync(key.as_bytes())?.is_some();

        Ok(is_marked)
    }

    pub fn start_delete_collection_sync(&self, collection_id: &str) -> Result<(), RawDbError> {
        // Now we need remove this collection from `Database.collections` and remove its raw_db,
        // cleanup collection data from meta_raw_db of `Database`
        // Order here matters, we need expect that process can crash in any moment,
        // after restart it should work and collection should be in one of states:
        //   - didn't deleted
        //   - deleted
        //   - marked as deleted (and then deleted on `Database::open`)

        // Mark for deletion, this will delete this collection on database open
        self.mark_collection_for_deletion_sync(collection_id)?;

        Ok(())
    }

    pub fn finish_delete_collection_sync(&self, collection_id: &str) -> Result<(), RawDbError> {
        // Remove from collections hashmap
        // It should happen in the end, to not accidentally create new collection with the same name
        // before raw_db files are removed
        let mut collections_lock = self.collections.blocking_write();
        collections_lock.remove(collection_id);
        drop(collections_lock);

        self.unmark_collection_for_deletion_sync(collection_id)?;

        Ok(())
    }

    pub fn finish_delete_collection_with_collections(
        &self,
        collections: &mut HashMap<String, Arc<Collection>>,
        collection_id: &str,
    ) -> Result<(), RawDbError> {
        // Remove from collections hashmap
        // It should happen in the end, to not accidentally create new collection with the same name
        // before raw_db files are removed
        collections.remove(collection_id);

        self.unmark_collection_for_deletion_sync(collection_id)?;

        Ok(())
    }
}
