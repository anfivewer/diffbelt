use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::OwnedGenerationId;
use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, StartManualGenerationIdTask,
};
use crate::util::async_sync_call::async_sync_call;

pub struct StartGenerationOptions {
    pub generation_id: OwnedGenerationId,
    pub abort_outdated: bool,
}

impl Collection {
    pub async fn start_generation(
        &self,
        options: StartGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let StartGenerationOptions {
            generation_id,
            abort_outdated,
        } = options;

        if !self.is_manual {
            return Err(CollectionMethodError::UnsupportedOperationForThisCollectionType);
        }

        let _: () = async_sync_call(|sender| {
            self.database_inner.add_generations_task(
                DatabaseCollectionGenerationsTask::StartManualGenerationId(
                    StartManualGenerationIdTask {
                        collection_id: self.generations_id,
                        sender,
                        next_generation_id: generation_id,
                        abort_outdated,
                    },
                ),
            )
        })
        .await??;

        Ok(())
    }
}
