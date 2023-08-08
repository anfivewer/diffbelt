use crate::util::indexed_container::{
    IndexedContainer, IndexedContainerItem, IndexedContainerPointer,
};

use std::collections::VecDeque;

use tokio::sync::{mpsc, oneshot};

#[derive(Copy, Clone)]
pub struct AsyncLockInstanceId {
    index: usize,
    counter: u64,
}

pub struct AsyncLockInstance<T> {
    id: AsyncLockInstanceId,
    data_and_drop_sender: Option<(T, oneshot::Sender<AsyncLockInstanceId>, oneshot::Sender<T>)>,
}

impl<T> AsyncLockInstance<T> {
    pub fn data(&self) -> &T {
        let (data, _, _) = self.data_and_drop_sender.as_ref().unwrap();

        data
    }

    pub fn data_mut(&mut self) -> &mut T {
        let (data, _, _) = self.data_and_drop_sender.as_mut().unwrap();

        data
    }
}

impl<T> Drop for AsyncLockInstance<T> {
    fn drop(&mut self) {
        if let Some((data, id_sender, data_sender)) = self.data_and_drop_sender.take() {
            id_sender.send(self.id).unwrap_or(());
            data_sender.send(data).unwrap_or(());
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

enum AsyncLockTask<T> {
    Lock {
        sender: oneshot::Sender<AsyncLockInstance<T>>,
        data: T,
        drop_sender: oneshot::Sender<T>,
    },
    ExclusiveLock {
        sender: oneshot::Sender<AsyncLockInstance<T>>,
        data: T,
        drop_sender: oneshot::Sender<T>,
    },
    Unlock {
        id: AsyncLockInstanceId,
    },
    Drop,
}

struct AsyncLockInternal<T> {
    is_locked_exclusively: bool,
    count: usize,
    limit: usize,
    locks: IndexedContainer<AsyncLockInstanceId>,
    waiters_for_lock: VecDeque<(oneshot::Sender<AsyncLockInstance<T>>, T, oneshot::Sender<T>)>,
    waiters_for_exclusive_lock:
        VecDeque<(oneshot::Sender<AsyncLockInstance<T>>, T, oneshot::Sender<T>)>,
    task_sender: mpsc::Sender<AsyncLockTask<T>>,
    task_receiver: mpsc::Receiver<AsyncLockTask<T>>,
}

impl<T: Send + 'static> AsyncLockInternal<T> {
    async fn run(mut self) {
        loop {
            let Some(task) = self.task_receiver.recv().await else {
                return;
            };

            match task {
                AsyncLockTask::Lock {
                    sender,
                    data,
                    drop_sender,
                } => {
                    let id_receiver = self.lock(sender, data, drop_sender);
                    self.locked(id_receiver);
                }
                AsyncLockTask::ExclusiveLock {
                    sender,
                    data,
                    drop_sender,
                } => {
                    let id_receiver = self.exclusive_lock(sender, data, drop_sender);
                    self.locked(id_receiver);
                }
                AsyncLockTask::Unlock { id } => {
                    self.unlock(id);
                }
                AsyncLockTask::Drop => {
                    return;
                }
            }
        }
    }

    fn lock(
        &mut self,
        sender: oneshot::Sender<AsyncLockInstance<T>>,
        data: T,
        drop_sender: oneshot::Sender<T>,
    ) -> Option<oneshot::Receiver<AsyncLockInstanceId>> {
        if self.limit > 0 && self.count >= self.limit || self.is_locked_exclusively {
            self.waiters_for_lock.push_back((sender, data, drop_sender));
            return None;
        }

        self.do_lock(sender, data, drop_sender)
    }

    fn exclusive_lock(
        &mut self,
        sender: oneshot::Sender<AsyncLockInstance<T>>,
        data: T,
        drop_sender: oneshot::Sender<T>,
    ) -> Option<oneshot::Receiver<AsyncLockInstanceId>> {
        if self.count > 0 {
            self.waiters_for_exclusive_lock
                .push_back((sender, data, drop_sender));
            return None;
        }

        self.is_locked_exclusively = true;

        self.do_lock(sender, data, drop_sender)
    }

    fn do_lock(
        &mut self,
        sender: oneshot::Sender<AsyncLockInstance<T>>,
        data: T,
        drop_sender: oneshot::Sender<T>,
    ) -> Option<oneshot::Receiver<AsyncLockInstanceId>> {
        self.count += 1;

        let (id_sender, id_receiver) = oneshot::channel();

        let id = self.locks.insert(|id| id);

        let instance = AsyncLockInstance {
            id: id.clone(),
            data_and_drop_sender: Some((data, id_sender, drop_sender)),
        };

        match sender.send(instance) {
            Ok(_) => Some(id_receiver),
            Err(_) => {
                self.unlock(id);
                None
            }
        }
    }

    fn locked(&self, id_receiver: Option<oneshot::Receiver<AsyncLockInstanceId>>) {
        let Some(id_receiver) = id_receiver else {
            return;
        };

        let task_sender = self.task_sender.clone();

        tokio::spawn(async move {
            let Ok(id) = id_receiver.await else {
                return;
            };

            task_sender
                .send(AsyncLockTask::Unlock { id })
                .await
                .unwrap_or(());
        });
    }

    fn unlock(&mut self, id: AsyncLockInstanceId) {
        let Some(_) = self.locks.delete(&id) else {
            // Already unlocked
            return;
        };

        self.is_locked_exclusively = false;
        self.count -= 1;

        if !self.waiters_for_exclusive_lock.is_empty() {
            if self.count == 0 {
                let (sender, data, drop_sender) =
                    self.waiters_for_exclusive_lock.pop_front().unwrap();
                self.is_locked_exclusively = true;
                self.do_lock(sender, data, drop_sender);
                return;
            }

            // force to queue all locks
            self.is_locked_exclusively = true;
            return;
        }

        if let Some((sender, data, drop_sender)) = self.waiters_for_lock.pop_front() {
            self.do_lock(sender, data, drop_sender);
        }
    }
}

pub struct AsyncLock<T> {
    lock_tasks_sender: mpsc::Sender<AsyncLockTask<T>>,
    drop_sender: Option<oneshot::Sender<()>>,
}

impl<T: Send + 'static> AsyncLock<T> {
    pub fn with_limit(limit: usize) -> Self {
        let (lock_tasks_sender, task_receiver) = mpsc::channel(16);

        let inner = AsyncLockInternal {
            is_locked_exclusively: false,
            count: 0,
            limit,
            locks: if limit >= 512 {
                IndexedContainer::new()
            } else {
                IndexedContainer::with_capacity(limit)
            },
            waiters_for_lock: VecDeque::new(),
            waiters_for_exclusive_lock: VecDeque::new(),
            task_sender: lock_tasks_sender.clone(),
            task_receiver,
        };

        tokio::spawn(async move {
            inner.run().await;
        });

        let (drop_sender, drop_receiver) = oneshot::channel();

        {
            let lock_tasks_sender = lock_tasks_sender.clone();
            tokio::spawn(async move {
                drop_receiver.await.unwrap_or(());

                lock_tasks_sender
                    .send(AsyncLockTask::Drop)
                    .await
                    .unwrap_or(());
            });
        }

        Self {
            lock_tasks_sender,
            drop_sender: Some(drop_sender),
        }
    }

    pub async fn lock(
        &self,
        data: T,
        drop_sender: oneshot::Sender<T>,
    ) -> Option<AsyncLockInstance<T>> {
        let (sender, receiver) = oneshot::channel();

        self.lock_tasks_sender
            .send(AsyncLockTask::Lock {
                sender,
                data,
                drop_sender,
            })
            .await
            .unwrap_or(());

        receiver.await.ok()
    }

    pub async fn exclusive_lock(
        &self,
        data: T,
        drop_sender: oneshot::Sender<T>,
    ) -> Option<AsyncLockInstance<T>> {
        let (sender, receiver) = oneshot::channel();

        self.lock_tasks_sender
            .send(AsyncLockTask::ExclusiveLock {
                sender,
                data,
                drop_sender,
            })
            .await
            .unwrap_or(());

        receiver.await.ok()
    }
}

impl<T> Drop for AsyncLock<T> {
    fn drop(&mut self) {
        if let Some(sender) = self.drop_sender.take() {
            sender.send(()).unwrap_or(());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::async_lock::AsyncLock;
    use crate::util::tokio_runtime::create_main_tokio_runtime;
    use futures::StreamExt;
    use std::io::Write;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;
    use tokio::sync::oneshot;
    use tokio::time::sleep;

    #[test]
    pub fn should_wait_if_exceeds_limit() {
        let runtime = create_main_tokio_runtime().unwrap();
        runtime.block_on(should_wait_if_exceeds_limit_inner());
    }

    async fn should_wait_if_exceeds_limit_inner() {
        let lock = Arc::new(AsyncLock::with_limit(2));

        let (sender1, receiver1) = oneshot::channel();
        let (sender2, receiver2) = oneshot::channel();
        let (sender3, receiver3) = oneshot::channel();

        let instance1 = lock.lock(1, sender1).await.unwrap();
        let instance2 = lock.lock(2, sender2).await.unwrap();

        let instance3_rw = Arc::new(RwLock::new(None));

        let spawn_handle = {
            let lock = lock.clone();
            let instance3_rw = instance3_rw.clone();
            tokio::spawn(async move {
                let instance3 = lock.lock(3, sender3).await.unwrap();

                let mut instance3_rw = instance3_rw.write().unwrap();
                instance3_rw.replace(instance3);
            })
        };

        sleep(Duration::from_millis(500)).await;

        assert!({ instance3_rw.read().unwrap().is_none() });

        drop(instance1);

        spawn_handle.await.unwrap();

        let instance3 = instance3_rw.write().unwrap().take().unwrap();

        let instance1_value = receiver1.await.unwrap();
        assert_eq!(instance1_value, 1);

        drop(instance2);
        drop(instance3);

        let instance2_value = receiver2.await.unwrap();
        assert_eq!(instance2_value, 2);

        let instance3_value = receiver3.await.unwrap();
        assert_eq!(instance3_value, 3);
    }
}
