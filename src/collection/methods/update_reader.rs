use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::OwnedGenerationId;
use crate::messages::readers::{DatabaseCollectionReadersTask, UpdateReaderTask};
use crate::raw_db::update_reader::RawDbUpdateReaderOptions;
use std::sync::Arc;
use tokio::task::spawn_blocking;

pub struct UpdateReaderOptions {
    pub reader_name: String,
    pub generation_id: Option<OwnedGenerationId>,
}

impl Collection {
    pub async fn update_reader(
        &self,
        options: UpdateReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        let reader_name = Arc::from(options.reader_name);
        let generation_id = Arc::new(options.generation_id.unwrap_or(OwnedGenerationId::empty()));
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let reader_name_for_blocking = Arc::clone(&reader_name);
        let generation_id_for_blocking = Arc::clone(&generation_id);

        let _: () = spawn_blocking(move || {
            raw_db.update_reader_sync(RawDbUpdateReaderOptions {
                reader_name: &reader_name_for_blocking,
                generation_id: OwnedGenerationId::as_ref(&generation_id_for_blocking),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        self.database_inner
            .add_readers_task(DatabaseCollectionReadersTask::UpdateReader(
                UpdateReaderTask {
                    owner_collection_name: self.name.clone(),
                    to_collection_name: None,
                    reader_name,
                    generation_id,
                },
            ))
            .await;

        drop(deletion_lock);

        Ok(())
    }
}
