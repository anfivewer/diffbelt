use crate::common::{IsByteArray, OwnedGenerationId};
use crate::database::generations::next_generation_lock::{
    GenerationIdLock, NextGenerationIdLockData,
};
use crate::util::async_lock::AsyncLock;
use crate::util::indexed_container::{IndexedContainerItem, IndexedContainerPointer};

use std::future::Future;
use std::sync::Arc;

use crate::collection::CommitGenerationUpdateReader;
use crate::messages::generations::{
    CommitManualGenerationError, LockManualGenerationIdError, StartManualGenerationIdError,
};
use crate::raw_db::commit_generation::{RawDbCommitGenerationOptions, RawDbUpdateReader};
use crate::raw_db::RawDbError;

use crate::collection::constants::COLLECTION_CF_META;
use crate::collection::methods::abort_generation::{
    abort_generation_sync, AbortGenerationSyncOptions,
};
use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::database::DatabaseInner;
use crate::messages::readers::{
    DatabaseCollectionReadersTask, UpdateReaderTask, UpdateReadersTask,
};
use crate::raw_db::has_generation_changes::HasGenerationChangesOptions;
use tokio::sync::{oneshot, watch, RwLock};
use tokio::task::spawn_blocking;

#[derive(Copy, Clone)]
pub struct InnerGenerationsCollectionId {
    pub index: usize,
    pub counter: u64,
}

#[derive(Clone)]
pub struct GenerationIdNextGenerationIdPair {
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
}

pub struct InnerGenerationsCollection {
    pub inner_id: InnerGenerationsCollectionId,
    name: Arc<str>,
    pub is_manual: bool,
    db: CollectionRawDb,
    scheduled_for_generation_id: Option<OwnedGenerationId>,
    pub generation_pair_sender: Arc<watch::Sender<GenerationIdNextGenerationIdPair>>,
    pub generation_pair_receiver: watch::Receiver<GenerationIdNextGenerationIdPair>,
    pub next_generation_locks:
        AsyncLock<GenerationIdNextGenerationIdPair, NextGenerationIdLockData>,
    pub is_next_generation_scheduled: bool,
    pub is_deleted: Arc<RwLock<bool>>,
}

pub enum NextGenerationScheduleAction {
    NeedSchedule,
    NoNeedSchedule,
}

pub struct NextGenerationLocked {
    pub next_generation_id: OwnedGenerationId,
    pub lock: GenerationIdLock,
    pub unlock_receiver: oneshot::Receiver<NextGenerationIdLockData>,
}

impl InnerGenerationsCollection {
    pub fn new(
        inner_id: InnerGenerationsCollectionId,
        name: Arc<str>,
        is_manual: bool,
        db: CollectionRawDb,
        generation_id: OwnedGenerationId,
        next_generation_id: Option<OwnedGenerationId>,
        is_deleted: Arc<RwLock<bool>>,
    ) -> Self {
        let (generation_id, next_generation_id) = {
            if is_manual {
                (generation_id, next_generation_id)
            } else {
                let next_generation_id = generation_id.incremented();
                (generation_id, Some(next_generation_id))
            }
        };

        let generation_pair = GenerationIdNextGenerationIdPair {
            generation_id,
            next_generation_id,
        };

        let (generation_pair_sender, generation_pair_receiver) =
            watch::channel(generation_pair.clone());

        Self {
            inner_id,
            name,
            is_manual,
            db,
            // generation_id: generation_id.clone(),
            // next_generation_id: next_generation_id.clone(),
            scheduled_for_generation_id: None,
            generation_pair_sender: Arc::new(generation_pair_sender),
            generation_pair_receiver,
            // TODO: move to config
            next_generation_locks: AsyncLock::with_limit(generation_pair, 64),
            // TODO: check after restart
            is_next_generation_scheduled: false,
            is_deleted,
        }
    }

    pub fn is_need_to_schedule_generation(
        &self,
        next_generation_id: OwnedGenerationId,
    ) -> impl Future<Output = Result<NextGenerationScheduleAction, RawDbError>> {
        let raw_db = self.db.clone();

        async move {
            spawn_blocking(move || {
                let has_changes =
                    raw_db.has_generation_changes_sync(HasGenerationChangesOptions {
                        generation_id: next_generation_id.as_ref(),
                    })?;

                if has_changes {
                    Ok(NextGenerationScheduleAction::NeedSchedule)
                } else {
                    Ok(NextGenerationScheduleAction::NoNeedSchedule)
                }
            })
            .await
            .map_err(RawDbError::Join)?
        }
    }

    pub fn start_manual_generation(
        &mut self,
        new_next_generation_id: OwnedGenerationId,
        abort_outdated: bool,
    ) -> impl Future<Output = Result<(), StartManualGenerationIdError>> {
        let next_generation_locks = self.next_generation_locks.mirror();
        let raw_db = self.db.clone();
        let is_deleted = self.is_deleted.clone();

        return async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            let comparison = pair
                .next_generation_id
                .as_ref()
                .map(|id| id.as_ref() == new_next_generation_id.as_ref());
            let is_equal = comparison.unwrap_or(false);
            let is_empty = comparison.is_none();

            if is_equal {
                return Ok(());
            }

            let mut need_abort_generation_id = None;

            if !is_empty {
                if abort_outdated {
                    if new_next_generation_id.as_ref()
                        > pair.next_generation_id.as_ref().unwrap().as_ref()
                    {
                        need_abort_generation_id = Some(pair.next_generation_id.clone().unwrap());
                    } else {
                        return Err(StartManualGenerationIdError::OutdatedGeneration);
                    }
                } else {
                    return Err(StartManualGenerationIdError::OutdatedGeneration);
                };
            }

            let new_next_generation_id_for_db = new_next_generation_id.clone();
            let _: () = spawn_blocking(move || {
                let is_deleted = is_deleted.blocking_read();
                if *is_deleted {
                    return Err(StartManualGenerationIdError::NoSuchCollection);
                }

                if let Some(need_abort_generation_id) = need_abort_generation_id {
                    let err = abort_generation_sync(AbortGenerationSyncOptions {
                        raw_db: raw_db.as_ref(),
                        generation_id: need_abort_generation_id.as_ref(),
                    });

                    match err {
                        Some(err) => {
                            return Err(StartManualGenerationIdError::RawDb(err));
                        }
                        None => {}
                    }
                }

                raw_db
                    .put_cf_sync(
                        COLLECTION_CF_META,
                        b"next_generation_id",
                        new_next_generation_id_for_db.get_byte_array(),
                    )
                    .map_err(StartManualGenerationIdError::RawDb)
            })
            .await
            .map_err(|error| StartManualGenerationIdError::RawDb(RawDbError::Join(error)))??;

            pair.next_generation_id = Some(new_next_generation_id.clone());

            Ok(())
        };
    }

    pub fn lock_next_generation(
        &mut self,
        expected_next_generation_id: Option<OwnedGenerationId>,
        is_phantom: bool,
    ) -> impl Future<Output = Result<NextGenerationLocked, LockManualGenerationIdError>> {
        let is_manual = self.is_manual;
        let next_generation_locks = self.next_generation_locks.mirror();

        async move {
            let (sender, receiver) = oneshot::channel();

            let lock = next_generation_locks
                .lock(NextGenerationIdLockData::new(), sender)
                .await;

            if is_phantom {
                let Some(expected_next_generation_id) = expected_next_generation_id else {
                    return Err(LockManualGenerationIdError::PutPhantomWithoutGenerationId);
                };

                return Ok(NextGenerationLocked {
                    next_generation_id: expected_next_generation_id,
                    lock: GenerationIdLock {
                        async_lock_instance: lock,
                    },
                    unlock_receiver: receiver,
                });
            }

            if !is_manual {
                return Ok(NextGenerationLocked {
                    next_generation_id: lock.value().next_generation_id.clone().unwrap(),
                    lock: GenerationIdLock {
                        async_lock_instance: lock,
                    },
                    unlock_receiver: receiver,
                });
            }

            let Some(expected_next_generation_id) = expected_next_generation_id else {
                return Err(LockManualGenerationIdError::GenerationIdMismatch);
            };

            let Some(next_generation_id) = lock.value().next_generation_id.as_ref().map(|id| id.as_ref()) else {
                return Err(LockManualGenerationIdError::GenerationIdMismatch);
            };

            if next_generation_id != expected_next_generation_id.as_ref() {
                return Err(LockManualGenerationIdError::GenerationIdMismatch);
            }

            Ok(NextGenerationLocked {
                next_generation_id: next_generation_id.to_owned(),
                lock: GenerationIdLock {
                    async_lock_instance: lock,
                },
                unlock_receiver: receiver,
            })
        }
    }

    pub fn schedule_next_generation(
        &mut self,
        expected_generation_id: OwnedGenerationId,
    ) -> NextGenerationScheduleAction {
        if self.is_manual || self.scheduled_for_generation_id.is_some() {
            return NextGenerationScheduleAction::NoNeedSchedule;
        }

        self.scheduled_for_generation_id = Some(expected_generation_id);

        NextGenerationScheduleAction::NeedSchedule
    }

    pub fn commit_next_generation(&mut self) {
        let Some(expected_generation_id) = self.scheduled_for_generation_id.take() else {
            return;
        };

        let next_generation_locks = self.next_generation_locks.mirror();
        let generation_pair_sender = self.generation_pair_sender.clone();
        let raw_db = self.db.clone();
        let is_deleted = self.is_deleted.clone();

        tokio::spawn(async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            if pair.generation_id != expected_generation_id {
                return;
            }

            pair.generation_id = pair.next_generation_id.take().unwrap();
            pair.next_generation_id = Some(pair.generation_id.incremented());
            let generation_id = pair.generation_id.clone();

            drop(lock);

            // Retaking lock to not block reads while we'll write to db
            let lock = next_generation_locks.lock_without_data().await;

            if lock.value().generation_id != generation_id {
                // We are too late, there is already next generation
                return;
            }

            // TODO: break only this collection, not whole server. Or at least shutdown gracefully
            let generation_id_for_db = generation_id.clone();
            let _: () = spawn_blocking(move || {
                let is_deleted = is_deleted.blocking_read();
                if *is_deleted {
                    return Err(CommitManualGenerationError::NoSuchCollection);
                }

                raw_db
                    .commit_generation_sync(RawDbCommitGenerationOptions {
                        generation_id: generation_id_for_db.as_ref(),
                        next_generation_id: OwnedGenerationId::empty().as_ref(),
                        update_readers: None,
                    })
                    .map_err(CommitManualGenerationError::RawDb)
            })
            .await
            .expect("commit_next_generation:join_error")
            .expect("commit_next_generation:raw_db_error");

            generation_pair_sender
                .send(lock.value().clone())
                .unwrap_or(());
        });
    }

    pub fn abort_manual_generation(
        &mut self,
        next_generation_id: OwnedGenerationId,
    ) -> impl Future<Output = Result<(), CommitManualGenerationError>> {
        let next_generation_locks = self.next_generation_locks.mirror();
        let generation_pair_sender = self.generation_pair_sender.clone();
        let raw_db = self.db.clone();
        let is_deleted = self.is_deleted.clone();

        async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            let is_equal = pair
                .next_generation_id
                .as_ref()
                .map(|id| id.as_ref() == next_generation_id.as_ref())
                .unwrap_or(false);

            if !is_equal {
                return Err(CommitManualGenerationError::OutdatedGeneration);
            }

            let generation_id_for_db = pair.generation_id.clone();
            let _: () = spawn_blocking(move || {
                let is_deleted = is_deleted.blocking_read();
                if *is_deleted {
                    return Err(CommitManualGenerationError::NoSuchCollection);
                }

                let err = abort_generation_sync(AbortGenerationSyncOptions {
                    raw_db: raw_db.as_ref(),
                    generation_id: next_generation_id.as_ref(),
                });

                if let Some(err) = err {
                    return Err(CommitManualGenerationError::RawDb(err));
                };

                raw_db
                    .commit_generation_sync(RawDbCommitGenerationOptions {
                        generation_id: generation_id_for_db.as_ref(),
                        next_generation_id: OwnedGenerationId::empty().as_ref(),
                        update_readers: None,
                    })
                    .map_err(CommitManualGenerationError::RawDb)
            })
            .await
            .map_err(|error| CommitManualGenerationError::RawDb(RawDbError::Join(error)))??;

            pair.next_generation_id.take();

            generation_pair_sender.send(pair.clone()).unwrap_or(());

            Ok(())
        }
    }

    pub fn commit_manual_generation(
        &mut self,
        database: Arc<DatabaseInner>,
        next_generation_id: OwnedGenerationId,
        update_readers: Option<Vec<CommitGenerationUpdateReader>>,
    ) -> impl Future<Output = Result<(), CommitManualGenerationError>> {
        let next_generation_locks = self.next_generation_locks.mirror();
        let generation_pair_sender = self.generation_pair_sender.clone();
        let raw_db = self.db.clone();
        let is_deleted = self.is_deleted.clone();
        let name = self.name.clone();

        async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            let is_equal = pair
                .next_generation_id
                .as_ref()
                .map(|id| id.as_ref() == next_generation_id.as_ref())
                .unwrap_or(false);

            if !is_equal {
                return Err(CommitManualGenerationError::OutdatedGeneration);
            }

            let update_readers_for_readers_thread = update_readers.as_ref().map(|update_readers| {
                update_readers
                    .iter()
                    .map(
                        |CommitGenerationUpdateReader {
                             reader_name,
                             generation_id,
                         }| UpdateReaderTask {
                            owner_collection_name: name.clone(),
                            to_collection_name: None,
                            reader_name: Arc::from(reader_name.as_str()),
                            generation_id: generation_id.clone(),
                        },
                    )
                    .collect()
            });

            let generation_id_for_db = next_generation_id.clone();
            let _: () = spawn_blocking(move || {
                let is_deleted = is_deleted.blocking_read();
                if *is_deleted {
                    return Err(CommitManualGenerationError::NoSuchCollection);
                }

                raw_db
                    .commit_generation_sync(RawDbCommitGenerationOptions {
                        generation_id: generation_id_for_db.as_ref(),
                        next_generation_id: OwnedGenerationId::empty().as_ref(),
                        update_readers: update_readers.as_ref().map(|update_readers| {
                            update_readers
                                .iter()
                                .map(
                                    |CommitGenerationUpdateReader {
                                         reader_name,
                                         generation_id,
                                     }| RawDbUpdateReader {
                                        reader_name: reader_name.as_str(),
                                        generation_id: generation_id.as_ref(),
                                    },
                                )
                                .collect()
                        }),
                    })
                    .map_err(CommitManualGenerationError::RawDb)
            })
            .await
            .map_err(|error| CommitManualGenerationError::RawDb(RawDbError::Join(error)))??;

            if let Some(update_readers_for_readers_thread) = update_readers_for_readers_thread {
                database
                    .add_readers_task(DatabaseCollectionReadersTask::UpdateReaders(
                        UpdateReadersTask {
                            updates: update_readers_for_readers_thread,
                        },
                    ))
                    .await;
            }

            pair.generation_id = next_generation_id.clone();
            pair.next_generation_id.take();

            generation_pair_sender.send(pair.clone()).unwrap_or(());

            Ok(())
        }
    }
}

impl IndexedContainerItem for InnerGenerationsCollection {
    type Item = InnerGenerationsCollection;
    type Id = InnerGenerationsCollectionId;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        InnerGenerationsCollectionId { index, counter }
    }
}

impl IndexedContainerPointer for InnerGenerationsCollectionId {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl IndexedContainerPointer for InnerGenerationsCollection {
    fn index(&self) -> usize {
        self.inner_id.index
    }

    fn counter(&self) -> u64 {
        self.inner_id.counter
    }
}
