use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::{Collection, CommitGenerationUpdateReader};
use crate::common::OwnedGenerationId;
use crate::messages::generations::{
    CommitManualGenerationError, CommitManualGenerationTask, DatabaseCollectionGenerationsTask,
};
use crate::messages::readers::{DatabaseCollectionReadersTask, GetMinimumGenerationIdLocksTask};
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

        let minimum_generation_id_locks = if let Some(update_readers) = &update_readers {
            let collection_name = self.name.clone();
            let mut reader_names = Vec::new();

            for update in update_readers {
                reader_names.push(update.reader_name.clone());
            }

            let locks = async_sync_call(|sender| {
                self.database_inner.add_readers_task(
                    DatabaseCollectionReadersTask::GetMinimumGenerationIdLocks(
                        GetMinimumGenerationIdLocksTask {
                            collection_name,
                            reader_names,
                            sender,
                        },
                    ),
                )
            })
            .await?;

            for update in update_readers {
                if let Some((minimum_generation_id, _)) = locks
                    .minimum_generation_ids_with_locks
                    .get(&update.reader_name)
                {
                    if &update.generation_id < minimum_generation_id {
                        return Err(CollectionMethodError::GenerationIdLessThanMinimum);
                    }
                }
            }

            Some(locks)
        } else {
            None
        };

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
            CommitManualGenerationError::NoSuchCollection => {
                CollectionMethodError::NoSuchCollection
            }
        })?;

        drop(minimum_generation_id_locks);

        Ok(())
    }
}
