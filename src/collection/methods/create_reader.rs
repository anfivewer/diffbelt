use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{GenerationId, OwnedGenerationId};
use crate::messages::readers::{DatabaseCollectionReadersTask, UpdateReaderTask};
use std::sync::Arc;
use tokio::task::spawn_blocking;

use crate::raw_db::update_reader::{RawDbCreateReaderOptions, RawDbCreateReaderResult};

pub struct CreateReaderOptions {
    pub reader_name: String,
    pub collection_name: Option<String>,
    pub generation_id: Option<OwnedGenerationId>,
}

impl Collection {
    pub async fn create_reader(
        &self,
        options: CreateReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        let reader_name = Arc::from(options.reader_name);
        let to_collection_name = options.collection_name.map(Arc::from);
        let generation_id = options.generation_id;
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let reader_name_for_blocking = Arc::clone(&reader_name);
        let to_collection_name_for_blocking = to_collection_name.as_ref().map(|x| Arc::clone(x));
        let generation_id_for_blocking = generation_id.clone();

        let result = spawn_blocking(move || {
            raw_db.create_reader_sync(RawDbCreateReaderOptions {
                reader_name: Arc::as_ref(&reader_name_for_blocking),
                collection_name: to_collection_name_for_blocking
                    .as_ref()
                    .map(|id| Arc::as_ref(id)),
                generation_id: generation_id_for_blocking
                    .as_ref()
                    .map(|x| OwnedGenerationId::as_ref(&x))
                    .unwrap_or(GenerationId::empty()),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        self.database_inner
            .add_readers_task(DatabaseCollectionReadersTask::UpdateReader(
                UpdateReaderTask {
                    owner_collection_name: self.name.clone(),
                    to_collection_name: Some(
                        to_collection_name.unwrap_or_else(|| self.name.clone()),
                    ),
                    reader_name,
                    generation_id: generation_id.unwrap_or(OwnedGenerationId::empty()),
                },
            ))
            .await;

        drop(deletion_lock);

        match result {
            RawDbCreateReaderResult::Created => Ok(()),
            RawDbCreateReaderResult::AlreadyExists(reader_value) => {
                Err(CollectionMethodError::ReaderAlreadyExists(reader_value))
            }
        }
    }
}
