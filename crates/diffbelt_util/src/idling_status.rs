use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::select;
use tokio::sync::watch;
use tokio::time::sleep;

use diffbelt_util_no_std::unique_value::UniqueValue;

#[derive(Clone)]
pub struct IdlingStatus {
    inner: Arc<Inner>,
    counter_changed_receiver: watch::Receiver<UniqueValue>,
}

struct Inner {
    counter: AtomicU64,
    change_sender: watch::Sender<UniqueValue>,
}

impl IdlingStatus {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(UniqueValue);

        Self {
            inner: Arc::new(Inner {
                counter: AtomicU64::new(0),
                change_sender: sender,
            }),
            counter_changed_receiver: receiver,
        }
    }

    pub fn idle(&self) -> bool {
        let value = self.inner.counter.load(Ordering::Relaxed);
        value == 0
    }

    pub fn busy(&self) -> bool {
        let value = self.inner.counter.load(Ordering::Relaxed);
        value > 0
    }

    pub async fn on_idle(&self) {
        let mut receiver = self.counter_changed_receiver.clone();

        loop {
            if self.idle() {
                return;
            }

            () = receiver
                .changed()
                .await
                .expect("cannot be dropped, it in self");
        }
    }

    pub async fn on_idle_for(&self, duration: Duration) {
        let mut receiver = self.counter_changed_receiver.clone();
        let mut is_first = true;

        loop {
            if self.idle() {
                if is_first {
                    is_first = false;
                } else {
                    return;
                }
            }

            () = select! {
                result = receiver.changed() => {
                    // require to run timeout again
                    is_first = true;
                    result.expect("cannot be dropped, it in self")
                },
                () = sleep(duration) => {
                    // will check for idle() and return if this is second-time check
                },
            };
        }
    }

    pub async fn on_busy(&self) {
        let mut receiver = self.counter_changed_receiver.clone();

        loop {
            if self.busy() {
                return;
            }

            () = receiver
                .changed()
                .await
                .expect("cannot be dropped, it in self");
        }
    }

    pub fn start_work(&self) -> BusyTask {
        BusyTask::new(self)
    }
}

pub struct BusyTask {
    inner: Arc<Inner>,
}

impl BusyTask {
    fn new(idling: &IdlingStatus) -> Self {
        let inner = idling.inner.clone();

        inner.counter.fetch_add(1, Ordering::Relaxed);
        () = inner.change_sender.send(UniqueValue).unwrap_or(());

        Self { inner }
    }
}

impl Drop for BusyTask {
    fn drop(&mut self) {
        self.inner.counter.fetch_sub(1, Ordering::Relaxed);
        () = self.inner.change_sender.send(UniqueValue).unwrap_or(());
    }
}
