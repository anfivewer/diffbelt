use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::newgen::commit_next_generation::{
    commit_next_generation_sync, CommitNextGenerationError, CommitNextGenerationSyncOptions,
};
use crate::collection::{Collection, CommitGenerationUpdateReader};
use crate::common::OwnedGenerationId;
use crate::util::tokio::spawn_blocking_async;

pub struct CommitGenerationOptions {
    pub generation_id: OwnedGenerationId,
    pub update_readers: Option<Vec<CommitGenerationUpdateReader>>,
}

impl Collection {
    pub async fn commit_generation(
        &self,
        options: CommitGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let raw_db = self.raw_db.clone();
        let generation_id_sender = self.generation_id_sender.clone();
        let generation_id = self.generation_id.clone();
        let next_generation_id = self.next_generation_id.clone();
        let is_manual_collection = self.is_manual;

        let CommitGenerationOptions {
            generation_id: expected_generation_id,
            update_readers,
        } = options;

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        spawn_blocking_async(async move {
            commit_next_generation_sync(CommitNextGenerationSyncOptions {
                expected_generation_id: Some(expected_generation_id),
                raw_db,
                generation_id_sender,
                generation_id,
                next_generation_id,
                is_manual_collection,
                update_readers,
            })
            .await
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))?
        .map_err(|err| match err {
            CommitNextGenerationError::RawDb(err) => CollectionMethodError::RawDb(err),
            CommitNextGenerationError::GenerationIdMismatch => {
                CollectionMethodError::OutdatedGeneration
            }
        })?;

        drop(deletion_lock);

        Ok(())
    }
}
