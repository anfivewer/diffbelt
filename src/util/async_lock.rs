use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};
use futures::future::BoxFuture;
use std::collections::VecDeque;
use tokio::sync::oneshot;

#[derive(Copy, Clone)]
pub struct AsyncLockInstanceId {
    index: usize,
    counter: u64,
}

pub struct AsyncLockInstance<T> {
    id: AsyncLockInstanceId,
    data_and_drop_sender: Option<(T, oneshot::Sender<T>)>,
}

impl <T> AsyncLockInstance<T> {
    pub fn data(&self) -> &T {
        let (data, _) = self.data_and_drop_sender.as_ref().unwrap();

        data
    }

    pub fn data_mut(&mut self) -> &mut T {
        let (data, _) = self.data_and_drop_sender.as_mut().unwrap();

        data
    }
}

impl<T> Drop for AsyncLockInstance<T> {
    fn drop(&mut self) {
        if let Some((data, sender)) = self.data_and_drop_sender.take() {
            sender.send(data).unwrap_or(());
        }
    }
}

impl IndexedContainerPointer for AsyncLockInstanceId {
    fn index(&self) -> usize {
        self.index
    }

    fn counter(&self) -> u64 {
        self.counter
    }
}

impl<T> IndexedContainerPointer for AsyncLockInstance<T> {
    fn index(&self) -> usize {
        self.id.index
    }

    fn counter(&self) -> u64 {
        self.id.counter
    }
}

impl IndexedContainerItem for AsyncLockInstanceId {
    type Item = AsyncLockInstanceId;
    type Id = AsyncLockInstanceId;

    fn new_id(index: usize, counter: u64) -> Self::Id {
        AsyncLockInstanceId { index, counter }
    }
}

pub struct AsyncLock<T> {
    limit: usize,
    count: usize,
    locks: IndexedContainer<AsyncLockInstanceId>,
    waiters_for_lock: VecDeque<Box<dyn FnOnce(&mut AsyncLock<T>)>>,
    waiters_for_exclusive_lock:
        VecDeque<Box<dyn FnOnce(&mut AsyncLock<T>) -> BoxFuture<'static, ()>>>,
}

impl<T: Send + 'static> AsyncLock<T> {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            limit,
            count: 0,
            locks: if limit >= 512 {
                IndexedContainer::new()
            } else {
                IndexedContainer::with_capacity(limit)
            },
            waiters_for_lock: VecDeque::new(),
            waiters_for_exclusive_lock: VecDeque::new(),
        }
    }

    fn lock_internal(&mut self, data: T) -> (AsyncLockInstance<T>, oneshot::Receiver<T>) {
        self.count += 1;

        let id = self.locks.insert(|id| id);

        let (sender, receiver) = oneshot::channel();

        (
            AsyncLockInstance {
                id,
                data_and_drop_sender: Some((data, sender)),
            },
            receiver,
        )
    }

    pub async fn lock<DropCallback: FnOnce(T) + Send + 'static>(
        &mut self,
        data: T,
        drop_callback: DropCallback,
    ) -> AsyncLockInstance<T> {
        let (instance, receiver) = if self.limit > 0 && self.count >= self.limit {
            let (sender, receiver) = oneshot::channel();

            self.waiters_for_lock.push_back(Box::new(move |lock| {
                sender.send(lock.lock_internal(data)).unwrap_or(());
            }));

            receiver.await.unwrap()
        } else {
            self.lock_internal(data)
        };

        tokio::spawn(async move {
            let data = receiver.await.unwrap();

            drop_callback(data);
        });

        instance
    }
}
