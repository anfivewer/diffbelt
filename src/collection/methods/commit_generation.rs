use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::newgen::commit_next_generation::{
    commit_next_generation_sync, CommitNextGenerationError, CommitNextGenerationSyncOptions,
};
use crate::collection::Collection;
use crate::common::{GenerationId, GenerationIdRef};
use crate::raw_db::remove_all_records_of_generation::RemoveAllRecordsOfGenerationSyncOptions;
use crate::raw_db::{RawDb, RawDbError};

pub struct CommitGenerationOptions {
    pub generation_id: GenerationId,
    // TODO: update readers
}

impl Collection {
    pub async fn commit_generation(
        &self,
        options: CommitGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let raw_db = self.raw_db.clone();
        let meta_raw_db = self.meta_raw_db.clone();
        let generation_id = self.generation_id.clone();
        let next_generation_id = self.next_generation_id.clone();

        let expected_generation_id = options.generation_id;

        tokio::task::spawn_blocking(move || {
            commit_next_generation_sync(CommitNextGenerationSyncOptions {
                expected_generation_id: Some(expected_generation_id),
                raw_db,
                meta_raw_db,
                generation_id,
                next_generation_id,
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))?
        .map_err(|err| match err {
            CommitNextGenerationError::RawDb(err) => CollectionMethodError::RawDb(err),
            CommitNextGenerationError::GenerationIdMissmatch => {
                CollectionMethodError::OutdatedGeneration
            }
        })?;

        Ok(())
    }
}
