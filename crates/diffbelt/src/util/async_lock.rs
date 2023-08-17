use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use tokio::sync::{oneshot, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

pub struct AsyncLockInstance<D, T> {
    lock: OwnedRwLockReadGuard<D>,
    data_and_drop_sender: Option<(T, oneshot::Sender<T>)>,
}

impl<D, T> AsyncLockInstance<D, T> {
    pub fn value(&self) -> &D {
        self.lock.deref()
    }

    pub fn data(&self) -> &T {
        let (data, _) = self.data_and_drop_sender.as_ref().unwrap();

        data
    }

    pub fn data_mut(&mut self) -> &mut T {
        let (data, _) = self.data_and_drop_sender.as_mut().unwrap();

        data
    }
}

impl<D, T> Drop for AsyncLockInstance<D, T> {
    fn drop(&mut self) {
        if let Some((data, data_sender)) = self.data_and_drop_sender.take() {
            data_sender.send(data).unwrap_or(());
        }
    }
}

pub struct AsyncLockExclusiveInstance<D, T> {
    lock: OwnedRwLockWriteGuard<D>,
    data_and_drop_sender: Option<(T, oneshot::Sender<T>)>,
}

impl<D, T> AsyncLockExclusiveInstance<D, T> {
    #[allow(dead_code)]
    pub fn value(&self) -> &D {
        self.lock.deref()
    }

    pub fn value_mut(&mut self) -> &mut D {
        self.lock.deref_mut()
    }

    #[allow(dead_code)]
    pub fn data(&self) -> &T {
        let (data, _) = self.data_and_drop_sender.as_ref().unwrap();

        data
    }

    #[allow(dead_code)]
    pub fn data_mut(&mut self) -> &mut T {
        let (data, _) = self.data_and_drop_sender.as_mut().unwrap();

        data
    }
}

impl<D, T> Drop for AsyncLockExclusiveInstance<D, T> {
    fn drop(&mut self) {
        if let Some((data, data_sender)) = self.data_and_drop_sender.take() {
            data_sender.send(data).unwrap_or(());
        }
    }
}

pub struct AsyncLock<D, T> {
    data: PhantomData<T>,
    value: Arc<RwLock<D>>,
}

impl<D: Send + Sync + 'static, T: Send + 'static> AsyncLock<D, T> {
    pub fn with_limit(value: D, limit: u32) -> Self {
        Self {
            data: PhantomData::default(),
            value: Arc::new(RwLock::with_max_readers(value, limit)),
        }
    }

    pub async fn lock(&self, data: T, drop_sender: oneshot::Sender<T>) -> AsyncLockInstance<D, T> {
        let lock = self.value.clone().read_owned().await;

        AsyncLockInstance {
            lock,
            data_and_drop_sender: Some((data, drop_sender)),
        }
    }

    pub async fn lock_without_data(&self) -> AsyncLockInstance<D, T> {
        let lock = self.value.clone().read_owned().await;

        AsyncLockInstance {
            lock,
            data_and_drop_sender: None,
        }
    }

    #[allow(dead_code)]
    pub async fn lock_exclusive(
        &self,
        data: T,
        drop_sender: oneshot::Sender<T>,
    ) -> AsyncLockExclusiveInstance<D, T> {
        let lock = self.value.clone().write_owned().await;

        AsyncLockExclusiveInstance {
            lock,
            data_and_drop_sender: Some((data, drop_sender)),
        }
    }

    pub async fn lock_exclusive_without_data(&self) -> AsyncLockExclusiveInstance<D, T> {
        let lock = self.value.clone().write_owned().await;

        AsyncLockExclusiveInstance {
            lock,
            data_and_drop_sender: None,
        }
    }

    pub fn mirror(&self) -> Self {
        Self {
            data: PhantomData::default(),
            value: self.value.clone(),
        }
    }
}
