use crate::common::{GenerationId, OwnedGenerationId};
use crate::database::generations::next_generation_lock::{
    NextGenerationIdLock, NextGenerationIdLockData,
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
use crate::raw_db::{RawDb, RawDbError};
use crate::util::bytes::increment;
use tokio::sync::{oneshot, watch};
use tokio::task::spawn_blocking;

#[derive(Copy, Clone)]
pub struct InnerGenerationsCollectionId {
    pub index: usize,
    pub counter: u64,
}

#[derive(Clone)]
pub struct GenerationIdNextGenerationIdPair {
    generation_id: OwnedGenerationId,
    next_generation_id: Option<OwnedGenerationId>,
}

pub struct InnerGenerationsCollection {
    pub inner_id: InnerGenerationsCollectionId,
    pub is_manual: bool,
    db: Arc<RawDb>,
    scheduled_for_generation_id: Option<OwnedGenerationId>,
    pub generation_id_sender: Arc<watch::Sender<OwnedGenerationId>>,
    pub generation_id_receiver: watch::Receiver<OwnedGenerationId>,
    pub next_generation_locks:
        AsyncLock<GenerationIdNextGenerationIdPair, NextGenerationIdLockData>,
    pub is_next_generation_scheduled: bool,
}

pub enum NextGenerationScheduleAction {
    NeedSchedule,
    NoNeedSchedule,
}

pub struct NextGenerationLocked {
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: OwnedGenerationId,
    pub lock: NextGenerationIdLock,
    pub unlock_receiver: oneshot::Receiver<NextGenerationIdLockData>,
}

impl InnerGenerationsCollection {
    pub fn new(
        inner_id: InnerGenerationsCollectionId,
        is_manual: bool,
        db: Arc<RawDb>,
        generation_id: OwnedGenerationId,
        next_generation_id: Option<OwnedGenerationId>,
    ) -> Self {
        let (generation_id_sender, generation_id_receiver) = watch::channel(generation_id.clone());

        Self {
            inner_id,
            is_manual,
            db,
            // generation_id: generation_id.clone(),
            // next_generation_id: next_generation_id.clone(),
            scheduled_for_generation_id: None,
            generation_id_sender: Arc::new(generation_id_sender),
            generation_id_receiver,
            // TODO: move to config
            next_generation_locks: AsyncLock::with_limit(
                GenerationIdNextGenerationIdPair {
                    generation_id,
                    next_generation_id,
                },
                64,
            ),
            // TODO: check after restart
            is_next_generation_scheduled: false,
        }
    }

    pub fn start_manual_generation(
        &mut self,
        new_next_generation_id: OwnedGenerationId,
    ) -> impl Future<Output = Result<(), StartManualGenerationIdError>> {
        let next_generation_locks = self.next_generation_locks.mirror();
        let raw_db = self.db.clone();

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
            if !is_empty {
                return Err(StartManualGenerationIdError::GenerationIdMismatch);
            }

            let generation_id = pair.generation_id.clone();

            // TODO: don't override generation_id
            let new_next_generation_id_for_db = new_next_generation_id.clone();
            let _: () = spawn_blocking(move || {
                raw_db.commit_generation_sync(RawDbCommitGenerationOptions {
                    generation_id: generation_id.as_ref(),
                    next_generation_id: new_next_generation_id_for_db.as_ref(),
                    update_readers: None,
                })
            })
            .await
            .map_err(|error| StartManualGenerationIdError::RawDb(RawDbError::Join(error)))?
            .map_err(StartManualGenerationIdError::RawDb)?;

            pair.next_generation_id = Some(new_next_generation_id.clone());

            Ok(())
        };
    }

    pub fn lock_next_generation(&mut self) -> impl Future<Output = NextGenerationLocked> {
        let (sender, receiver) = oneshot::channel();

        let next_generation_locks = self.next_generation_locks.mirror();

        async move {
            let async_lock_instance = next_generation_locks
                .lock(NextGenerationIdLockData::new(), sender)
                .await;

            let GenerationIdNextGenerationIdPair {
                generation_id,
                next_generation_id,
            } = { async_lock_instance.value().clone() };

            let next_generation_id =
                next_generation_id.unwrap_or_else(|| generation_id.incremented());

            NextGenerationLocked {
                generation_id,
                next_generation_id,
                lock: NextGenerationIdLock {
                    async_lock_instance,
                },
                unlock_receiver: receiver,
            }
        }
    }

    pub fn lock_manual_generation(
        &mut self,
        expected_next_generation_id: OwnedGenerationId,
    ) -> impl Future<Output = Result<NextGenerationLocked, LockManualGenerationIdError>> {
        let next_generation_locks = self.next_generation_locks.mirror();

        async move {
            let (sender, receiver) = oneshot::channel();

            let lock = next_generation_locks
                .lock(NextGenerationIdLockData::new(), sender)
                .await;

            let GenerationIdNextGenerationIdPair {
                generation_id,
                next_generation_id,
            } = { lock.value().clone() };

            let is_equal = next_generation_id
                .as_ref()
                .map(|id| id.as_ref() == expected_next_generation_id.as_ref())
                .unwrap_or(false);

            if !is_equal {
                return Err(LockManualGenerationIdError::GenerationIdMismatch);
            }

            Ok(NextGenerationLocked {
                generation_id,
                next_generation_id: expected_next_generation_id,
                lock: NextGenerationIdLock {
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
        let generation_id_sender = self.generation_id_sender.clone();
        let raw_db = self.db.clone();

        tokio::spawn(async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            if pair.generation_id != expected_generation_id {
                return;
            }

            pair.generation_id = pair.generation_id.incremented();
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
                raw_db.commit_generation_sync(RawDbCommitGenerationOptions {
                    generation_id: generation_id_for_db.as_ref(),
                    next_generation_id: OwnedGenerationId::empty().as_ref(),
                    update_readers: None,
                })
            })
            .await
            .expect("commit_next_generation:join_error")
            .expect("commit_next_generation:raw_db_error");

            generation_id_sender.send(generation_id).unwrap_or(());
        });
    }

    pub fn abort_manual_generation(
        &mut self,
        next_generation_id: OwnedGenerationId,
    ) -> impl Future<Output = Result<(), CommitManualGenerationError>> {
        let next_generation_locks = self.next_generation_locks.mirror();
        let generation_id_sender = self.generation_id_sender.clone();
        let raw_db = self.db.clone();

        async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            let is_equal = pair
                .next_generation_id
                .as_ref()
                .map(|id| id.as_ref() == next_generation_id.as_ref())
                .unwrap_or(false);

            if !is_equal {
                return Err(CommitManualGenerationError::GenerationIdMismatch);
            }

            let generation_id_for_db = pair.generation_id.clone();
            let _: () = spawn_blocking(move || {
                raw_db.commit_generation_sync(RawDbCommitGenerationOptions {
                    generation_id: generation_id_for_db.as_ref(),
                    next_generation_id: OwnedGenerationId::empty().as_ref(),
                    update_readers: None,
                })
            })
            .await
            .map_err(|error| CommitManualGenerationError::RawDb(RawDbError::Join(error)))?
            .map_err(CommitManualGenerationError::RawDb)?;

            pair.next_generation_id.take();

            Ok(())
        }
    }

    pub fn commit_manual_generation(
        &mut self,
        next_generation_id: OwnedGenerationId,
        update_readers: Option<Vec<CommitGenerationUpdateReader>>,
    ) -> impl Future<Output = Result<(), CommitManualGenerationError>> {
        let next_generation_locks = self.next_generation_locks.mirror();
        let generation_id_sender = self.generation_id_sender.clone();
        let raw_db = self.db.clone();

        async move {
            let mut lock = next_generation_locks.lock_exclusive_without_data().await;

            let pair = lock.value_mut();

            let is_equal = pair
                .next_generation_id
                .as_ref()
                .map(|id| id.as_ref() == next_generation_id.as_ref())
                .unwrap_or(false);

            if !is_equal {
                return Err(CommitManualGenerationError::GenerationIdMismatch);
            }

            let generation_id_for_db = next_generation_id.clone();
            let _: () = spawn_blocking(move || {
                raw_db.commit_generation_sync(RawDbCommitGenerationOptions {
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
            })
            .await
            .map_err(|error| CommitManualGenerationError::RawDb(RawDbError::Join(error)))?
            .map_err(CommitManualGenerationError::RawDb)?;

            pair.generation_id = next_generation_id.clone();
            pair.next_generation_id.take();

            generation_id_sender.send(next_generation_id).unwrap_or(());

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
