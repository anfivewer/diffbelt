use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{GenerationId, OwnedGenerationId};
use crate::raw_db::remove_all_records_of_generation::RemoveAllRecordsOfGenerationSyncOptions;
use crate::raw_db::{RawDb, RawDbError};

pub struct AbortGenerationOptions {
    pub generation_id: OwnedGenerationId,
}

impl Collection {
    pub async fn abort_generation(
        &self,
        options: AbortGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        tokio::task::spawn_blocking(move || {
            let err = abort_generation_sync(AbortGenerationSyncOptions {
                raw_db: raw_db.as_ref(),
                generation_id: options.generation_id.as_ref(),
            });

            match err {
                Some(err) => Err(err),
                None => Ok(()),
            }
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

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
