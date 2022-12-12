use crate::collection::methods::abort_generation::{
    abort_generation_sync, AbortGenerationSyncOptions,
};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{GenerationId, IsByteArray};

pub struct StartGenerationOptions {
    pub generation_id: GenerationId,
    pub abort_outdated: bool,
}

impl Collection {
    pub async fn start_generation(
        &self,
        options: StartGenerationOptions,
    ) -> Result<(), CollectionMethodError> {
        if !self.is_manual {
            return Err(CollectionMethodError::CannotStartGenerationInNonManualCollection);
        }

        let current_generation_id = self.generation_id.clone();

        let generation_id = options.generation_id;
        let abort_outdated = options.abort_outdated;

        let raw_db = self.raw_db.clone();
        let meta_raw_db = self.meta_raw_db.clone();

        let next_generation_id = self.next_generation_id.clone();

        tokio::task::spawn_blocking(move || {
            let mut next_generation_id_lock = next_generation_id.write().unwrap();
            let generation_id_lock = current_generation_id.read().unwrap();
            let current_generation_id = generation_id_lock.as_ref();

            if current_generation_id >= generation_id.as_ref() {
                return Err(CollectionMethodError::OutdatedGeneration);
            }

            let next_generation_id = next_generation_id_lock.as_ref();

            match next_generation_id {
                Some(next_generation_id) => {
                    if &generation_id == next_generation_id {
                        println!("same");
                        return Ok(());
                    }

                    if abort_outdated {
                        if &generation_id > next_generation_id {
                            let err = abort_generation_sync(AbortGenerationSyncOptions {
                                raw_db: raw_db.as_ref(),
                                generation_id: next_generation_id.as_ref(),
                            });

                            match err {
                                Some(err) => {
                                    return Err(CollectionMethodError::RawDb(err));
                                }
                                None => {}
                            }
                        } else {
                            return Err(CollectionMethodError::OutdatedGeneration);
                        }
                    } else {
                        return Err(CollectionMethodError::OutdatedGeneration);
                    };
                }
                None => {}
            };

            meta_raw_db.put_sync(b"next_generation_id", generation_id.get_byte_array())?;

            next_generation_id_lock.replace(generation_id);

            Ok(())
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        Ok(())
    }
}
