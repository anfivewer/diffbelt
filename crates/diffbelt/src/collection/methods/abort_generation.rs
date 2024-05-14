use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{GenerationId, OwnedGenerationId};
use crate::messages::generations::{AbortManualGenerationTask, DatabaseCollectionGenerationsTask};
use crate::raw_db::remove_all_records_of_generation::RemoveAllRecordsOfGenerationSyncOptions;
use crate::raw_db::{RawDb, RawDbError};
use crate::util::async_sync_call::async_sync_call;

pub struct AbortGenerationOptions {
    pub generation_id: OwnedGenerationId,
}

impl Collection {
    pub async fn abort_generation(
        &self,
        options: AbortGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let AbortGenerationOptions { generation_id } = options;

        let _: () = async_sync_call(|sender| {
            self.database_inner.add_generations_task(
                DatabaseCollectionGenerationsTask::AbortManualGeneration(
                    AbortManualGenerationTask {
                        collection_id: self.generations_id,
                        sender,
                        generation_id,
                    },
                ),
            )
        })
        .await??;

        Ok(())
    }
}

pub struct AbortGenerationSyncOptions<'a> {
    pub raw_db: &'a RawDb,
    pub generation_id: GenerationId<'a>,
}

pub fn abort_generation_sync(options: AbortGenerationSyncOptions<'_>) -> Option<RawDbError> {
    let raw_db = options.raw_db;
    let generation_id = options.generation_id;

    let result =
        raw_db.remove_all_records_of_generation_sync(RemoveAllRecordsOfGenerationSyncOptions {
            generation_id,
        });

    match result {
        Ok(_) => None,
        Err(err) => Some(err),
    }
}
