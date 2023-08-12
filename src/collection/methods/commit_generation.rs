use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::{Collection, CommitGenerationUpdateReader};
use crate::common::OwnedGenerationId;
use crate::messages::generations::{
    CommitManualGenerationError, CommitManualGenerationTask, DatabaseCollectionGenerationsTask,
};
use crate::util::async_sync_call::async_sync_call;

pub struct CommitGenerationOptions {
    pub generation_id: OwnedGenerationId,
    pub update_readers: Option<Vec<CommitGenerationUpdateReader>>,
}

impl Collection {
    pub async fn commit_generation(
        &self,
        options: CommitGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let CommitGenerationOptions {
            generation_id: expected_generation_id,
            update_readers,
        } = options;

        let _: () = async_sync_call(|sender| {
            self.database_inner.add_generations_task(
                DatabaseCollectionGenerationsTask::CommitManualGeneration(
                    CommitManualGenerationTask {
                        collection_id: self.generations_id,
                        sender,
                        generation_id: expected_generation_id,
                        update_readers,
                    },
                ),
            )
        })
        .await
        .map_err(CollectionMethodError::OneshotRecv)?
        .map_err(|err| match err {
            CommitManualGenerationError::RawDb(err) => CollectionMethodError::RawDb(err),
            CommitManualGenerationError::OutdatedGeneration => {
                CollectionMethodError::OutdatedGeneration
            }
        })?;

        Ok(())
    }
}
