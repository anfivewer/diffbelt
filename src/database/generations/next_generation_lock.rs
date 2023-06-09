use crate::util::indexed_container::{IndexedContainerItem, IndexedContainerPointer};
use tokio::sync::oneshot;

#[derive(Copy, Clone)]
pub struct NextGenerationIdLock {
    pub index: usize,
    pub counter: u64,
}

impl IndexedContainerPointer for NextGenerationIdLock {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl IndexedContainerItem for NextGenerationIdLock {
    type Item = NextGenerationIdLock;
    type Id = NextGenerationIdLock;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        NextGenerationIdLock { index, counter }
    }
}

pub struct NextGenerationIdLockWithSender {
    pub sender: Option<oneshot::Sender<()>>,
}

impl Drop for NextGenerationIdLockWithSender {
    fn drop(&mut self) {
        let Some(sender) = self.sender.take() else {
            return;
        };

        sender.send(()).unwrap_or(());
    }
}
