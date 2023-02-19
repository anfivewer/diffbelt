use crate::common::{IsByteArrayMut, OwnedGenerationId};
use crate::raw_db::commit_generation::{RawDbCommitGenerationOptions, RawDbUpdateReader};
use crate::raw_db::has_generation_changes::HasGenerationChangesOptions;

use crate::collection::CommitGenerationUpdateReader;
use crate::raw_db::{RawDb, RawDbError};
use crate::util::bytes::increment;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::sync::RwLock;

pub struct CommitNextGenerationSyncOptions {
    pub expected_generation_id: Option<OwnedGenerationId>,
    pub raw_db: Arc<RawDb>,
    pub generation_id_sender: Arc<watch::Sender<OwnedGenerationId>>,
    pub generation_id: Arc<RwLock<OwnedGenerationId>>,
    pub next_generation_id: Arc<RwLock<Option<OwnedGenerationId>>>,
    pub is_manual_collection: bool,
    pub update_readers: Option<Vec<CommitGenerationUpdateReader>>,
}

pub enum CommitNextGenerationError {
    GenerationIdMissmatch,
    RawDb(RawDbError),
}

pub async fn commit_next_generation_sync(
    options: CommitNextGenerationSyncOptions,
) -> Result<(), CommitNextGenerationError> {
    // Check that commit is required
    // In case of `expected_generation_id` presense acquire lock
    // and validate that next generation is equal to expected
    let (next_generation_id, next_generation_id_lock) = match options.expected_generation_id {
        Some(expected_generation_id) => {
            let next_generation_id_lock = options.next_generation_id.write().await;
            match next_generation_id_lock.as_ref() {
                Some(next_generation_id) => {
                    if &expected_generation_id != next_generation_id {
                        return Err(CommitNextGenerationError::GenerationIdMissmatch);
                    }

                    let next_generation_id = next_generation_id.clone();
                    (next_generation_id, Some(next_generation_id_lock))
                }
                None => {
                    return Err(CommitNextGenerationError::GenerationIdMissmatch);
                }
            }
        }
        None => {
            let next_generation_id = options.next_generation_id.read().await;
            match next_generation_id.as_ref() {
                Some(next_generation_id) => (next_generation_id.as_ref().to_owned(), None),
                None => {
                    return Ok(());
                }
            }
        }
    };

    let has_changes = options
        .raw_db
        .has_generation_changes_local(HasGenerationChangesOptions {
            generation_id: next_generation_id.as_ref(),
        })
        .map_err(|err| CommitNextGenerationError::RawDb(err))?;

    if !has_changes {
        return Ok(());
    }

    let new_next_generation_id = if options.is_manual_collection {
        OwnedGenerationId::empty()
    } else {
        let mut new_next_generation_id = next_generation_id.clone();
        increment(new_next_generation_id.get_byte_array_mut());
        new_next_generation_id
    };

    // Store new gens
    options
        .raw_db
        .commit_generation_sync(RawDbCommitGenerationOptions {
            generation_id: next_generation_id.as_ref(),
            next_generation_id: new_next_generation_id.as_ref(),
            update_readers: options.update_readers.as_ref().map(|update_readers| {
                update_readers
                    .iter()
                    .map(
                        |CommitGenerationUpdateReader {
                             reader_id,
                             generation_id,
                         }| RawDbUpdateReader {
                            reader_id: reader_id.as_str(),
                            generation_id: generation_id.as_ref(),
                        },
                    )
                    .collect()
            }),
        })
        .map_err(|err| CommitNextGenerationError::RawDb(err))?;

    // Lock next generation first to prevent more puts
    let mut next_generation_id_lock = match next_generation_id_lock {
        Some(lock) => lock,
        None => options.next_generation_id.write().await,
    };
    let mut generation_id_lock = options.generation_id.write().await;

    generation_id_lock.replace(next_generation_id.clone());

    if options.is_manual_collection {
        next_generation_id_lock.take();
    } else {
        next_generation_id_lock.replace(new_next_generation_id);
    }

    options
        .generation_id_sender
        .send_replace(next_generation_id);

    drop(generation_id_lock);
    drop(next_generation_id_lock);

    Ok(())
}
