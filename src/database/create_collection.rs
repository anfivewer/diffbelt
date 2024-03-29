use crate::collection::open::{CollectionOpenError, CollectionOpenOptions};
use crate::collection::Collection;
use crate::database::constants::DATABASE_RAW_DB_CF;
use crate::database::Database;
use crate::protos::database_meta::CollectionRecord;
use crate::raw_db::RawDbError;
use protobuf::Message;
use std::sync::Arc;

pub struct CreateCollectionOptions {
    pub is_manual: bool,
}

#[derive(Debug)]
pub enum CreateCollectionError {
    RawDb(RawDbError),
    AlreadyExist,
    Protobuf(protobuf::Error),
    CollectionOpen(CollectionOpenError),
}

const PREFIX: &[u8] = b"collection:";

impl Database {
    pub async fn get_collection(&self, id: &str) -> Option<Arc<Collection>> {
        let collections_lock = self.collections.read().await;
        let collection = collections_lock.get(id);

        collection.map(|collection| collection.clone())
    }

    #[inline]
    pub async fn create_collection(
        &self,
        id: &str,
        options: CreateCollectionOptions,
    ) -> Result<Arc<Collection>, CreateCollectionError> {
        self.create_collection_inner(id, &options).await
    }

    async fn create_collection_inner(
        &self,
        id: &str,
        options: &CreateCollectionOptions,
    ) -> Result<Arc<Collection>, CreateCollectionError> {
        // We don't want to lock `self.collections` for write while creating
        // new collection/saving record to meta_raw_db, so it's in a separate lock
        let guard = self.collections_alter_lock.lock().await;

        let collections = self.collections.read().await;
        if collections.contains_key(id) {
            return Err(CreateCollectionError::AlreadyExist);
        }
        drop(collections);

        let id_bytes = id.as_bytes();
        let mut meta_collection_record_key: Vec<u8> =
            Vec::with_capacity(PREFIX.len() + id_bytes.len());
        meta_collection_record_key.extend_from_slice(PREFIX);
        meta_collection_record_key.extend_from_slice(id_bytes);
        let meta_collection_record_key = meta_collection_record_key.as_slice();

        let mut collection_record = CollectionRecord::new();
        collection_record.id = id.to_string();
        collection_record.is_manual = options.is_manual;

        let collection_record = collection_record
            .write_to_bytes()
            .or_else(|err| Err(CreateCollectionError::Protobuf(err)))?;

        self.database_raw_db
            .put_cf(
                DATABASE_RAW_DB_CF,
                meta_collection_record_key,
                &collection_record,
            )
            .await
            .or_else(|err| Err(CreateCollectionError::RawDb(err)))?;

        let collection = Collection::open(CollectionOpenOptions {
            config: self.config.clone(),
            name: id.to_string(),
            is_manual: options.is_manual,
            data_path: &self.data_path,
            database_inner: self.inner.clone(),
        })
        .await
        .or_else(|err| Err(CreateCollectionError::CollectionOpen(err)))?;

        let mut collections = self.collections.write().await;
        collections.insert(id.to_string(), collection.clone());
        drop(collections);

        drop(guard);

        Ok(collection)
    }
}
