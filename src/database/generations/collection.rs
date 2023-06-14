use crate::common::OwnedGenerationId;
use crate::database::generations::next_generation_lock::NextGenerationIdLock;
use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};
use tokio::sync::watch;

#[derive(Copy, Clone)]
pub struct InnerGenerationsCollectionId {
    pub index: usize,
    pub counter: u64,
}

pub struct InnerGenerationsCollection {
    pub inner_id: InnerGenerationsCollectionId,
    pub is_manual: bool,
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
    pub generation_id_sender: watch::Sender<OwnedGenerationId>,
    pub generation_id_receiver: watch::Receiver<OwnedGenerationId>,
    pub next_generation_locks: IndexedContainer<NextGenerationIdLock>,
    pub is_next_generation_scheduled: bool,
}

pub enum NextGenerationScheduleAction {
    NeedSchedule,
    NoNeedSchedule,
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
            generation_id,
            next_generation_id,
            generation_id_sender,
            generation_id_receiver,
            // TODO: add size limit
            next_generation_locks: IndexedContainer::new(),
            // TODO: check after restart
            is_next_generation_scheduled: false,
        }
    }

    pub fn lock_next_generation(&mut self) -> NextGenerationIdLock {
        return self.next_generation_locks.insert(|id| id);
    }

    pub fn unlock_next_generation(&mut self, id: NextGenerationIdLock) {
        self.next_generation_locks.delete(&id);
    }

    pub fn schedule_next_generation(&mut self) -> NextGenerationScheduleAction {
        if self.is_manual || self.is_next_generation_scheduled {
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
