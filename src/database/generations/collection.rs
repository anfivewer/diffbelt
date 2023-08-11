use crate::common::{GenerationId, OwnedGenerationId};
use crate::database::generations::next_generation_lock::{
    NextGenerationIdLock, NextGenerationIdLockData,
};
use crate::util::async_lock::AsyncLock;
use crate::util::indexed_container::{IndexedContainerItem, IndexedContainerPointer};

use std::future::Future;

use tokio::sync::{oneshot, watch};

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
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
    pub generation_id_sender: watch::Sender<OwnedGenerationId>,
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
    pub next_generation_id: Option<OwnedGenerationId>,
    pub lock: NextGenerationIdLock,
    pub unlock_receiver: oneshot::Receiver<NextGenerationIdLockData>,
}

impl InnerGenerationsCollection {
    pub fn new(
        inner_id: InnerGenerationsCollectionId,
        is_manual: bool,
        generation_id: OwnedGenerationId,
        next_generation_id: Option<OwnedGenerationId>,
    ) -> Self {
        let (generation_id_sender, generation_id_receiver) = watch::channel(generation_id.clone());

        Self {
            inner_id,
            is_manual,
            generation_id: generation_id.clone(),
            next_generation_id: next_generation_id.clone(),
            generation_id_sender,
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

    pub fn schedule_next_generation(
        &mut self,
        expected_generation_id: GenerationId<'_>,
    ) -> NextGenerationScheduleAction {
        if self.is_manual || self.is_next_generation_scheduled {
            return NextGenerationScheduleAction::NoNeedSchedule;
        }

        if self.generation_id.as_ref() != expected_generation_id {
            return NextGenerationScheduleAction::NoNeedSchedule;
        }

        self.is_next_generation_scheduled = true;

        NextGenerationScheduleAction::NeedSchedule
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
