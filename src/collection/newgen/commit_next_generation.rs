use crate::common::{IsByteArray, IsByteArrayMut, OwnedGenerationId};
use crate::raw_db::has_generation_changes::HasGenerationChangesOptions;
use crate::raw_db::put::PutKeyValue;
use crate::raw_db::{RawDb, RawDbError};
use crate::util::bytes::increment;
use std::sync::Arc;

pub struct CommitNextGenerationSyncOptions {
    pub expected_generation_id: Option<OwnedGenerationId>,
    pub raw_db: Arc<RawDb>,
    pub meta_raw_db: Arc<RawDb>,
    pub generation_id: Arc<std::sync::RwLock<OwnedGenerationId>>,
    pub next_generation_id: Arc<std::sync::RwLock<Option<OwnedGenerationId>>>,
}

pub enum CommitNextGenerationError {
    GenerationIdMissmatch,
    RawDb(RawDbError),
}

pub fn commit_next_generation_sync(
    options: CommitNextGenerationSyncOptions,
) -> Result<(), CommitNextGenerationError> {
    // Check that commit is required
    // In case of `expected_generation_id` presense acquire lock
    // and validate that next generation is equal to expected
    let (next_generation_id, next_generation_id_lock) = match options.expected_generation_id {
        Some(expected_generation_id) => {
            let next_generation_id_lock = options.next_generation_id.write().unwrap();
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
            let next_generation_id = options.next_generation_id.read().unwrap();
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

    let mut new_next_generation_id = next_generation_id.clone();
    increment(new_next_generation_id.get_byte_array_mut());

    // Store new gens
    options
        .meta_raw_db
        .put_two_sync(
            PutKeyValue {
                key: b"generation_id",
                value: next_generation_id.get_byte_array(),
            },
            PutKeyValue {
                key: b"next_generation_id",
                value: new_next_generation_id.get_byte_array(),
            },
        )
        .map_err(|err| CommitNextGenerationError::RawDb(err))?;

    // Lock next generation first to prevent more puts
    let mut next_generation_id_lock = match next_generation_id_lock {
        Some(lock) => lock,
        None => options.next_generation_id.write().unwrap(),
    };
    let mut generation_id_lock = options.generation_id.write().unwrap();

    generation_id_lock.replace(next_generation_id);
    next_generation_id_lock.replace(new_next_generation_id);

    drop(generation_id_lock);
    drop(next_generation_id_lock);

    Ok(())
}
