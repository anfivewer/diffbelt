use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::newgen::commit_next_generation::{
    commit_next_generation_sync, CommitNextGenerationError, CommitNextGenerationSyncOptions,
};
use crate::collection::Collection;
use crate::common::OwnedGenerationId;
use crate::util::tokio::spawn_blocking_async;

pub struct CommitGenerationOptions {
    pub generation_id: OwnedGenerationId,
    // TODO: update readers
}

impl Collection {
    pub async fn commit_generation(
        &self,
        options: CommitGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let raw_db = self.raw_db.clone();
        let meta_raw_db = self.meta_raw_db.clone();
        let generation_id_sender = self.generation_id_sender.clone();
        let generation_id = self.generation_id.clone();
        let next_generation_id = self.next_generation_id.clone();
        let is_manual_collection = self.is_manual;

        let expected_generation_id = options.generation_id;

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        spawn_blocking_async(async move {
            commit_next_generation_sync(CommitNextGenerationSyncOptions {
                expected_generation_id: Some(expected_generation_id),
                raw_db,
                meta_raw_db,
                generation_id_sender,
                generation_id,
                next_generation_id,
                is_manual_collection,
            })
            .await
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))?
        .map_err(|err| match err {
            CommitNextGenerationError::RawDb(err) => CollectionMethodError::RawDb(err),
            CommitNextGenerationError::GenerationIdMissmatch => {
                CollectionMethodError::OutdatedGeneration
            }
        })?;

        drop(deletion_lock);

        Ok(())
    }
}
