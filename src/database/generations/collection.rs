use crate::common::OwnedGenerationId;
use crate::util::indexed_container::{IndexedContainerItem, IndexedContainerPointer};
use tokio::sync::watch;

#[derive(Copy, Clone)]
pub struct InnerGenerationsCollectionId {
    pub index: usize,
    pub counter: u64,
}

pub struct InnerGenerationsCollection {
    pub inner_id: InnerGenerationsCollectionId,
    pub generation_id: OwnedGenerationId,
    pub next_generation_id: Option<OwnedGenerationId>,
    pub generation_id_sender: watch::Sender<OwnedGenerationId>,
    pub generation_id_receiver: watch::Receiver<OwnedGenerationId>,
}

impl InnerGenerationsCollection {
    pub fn new(
        inner_id: InnerGenerationsCollectionId,
        generation_id: OwnedGenerationId,
        next_generation_id: Option<OwnedGenerationId>,
    ) -> Self {
        let (generation_id_sender, generation_id_receiver) = watch::channel(generation_id.clone());

        Self {
            inner_id,
            generation_id,
            next_generation_id,
            generation_id_sender,
            generation_id_receiver,
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
