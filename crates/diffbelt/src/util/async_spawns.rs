use std::future::Future;

use tokio::sync::watch;

pub fn watch_is_true_or_end<'a>(
    receiver: &'a mut watch::Receiver<bool>,
) -> impl Future<Output = ()> + 'a {
    async move {
        while receiver.changed().await.is_ok() {
            let is_stopped = *receiver.borrow();
            if is_stopped {
                break;
            }
        }
    }
}

pub fn run_when_watch_is_true_or_end(
    mut receiver: watch::Receiver<bool>,
    fut: impl Future<Output = ()> + Send + 'static,
) {
    tokio::spawn(async move {
        watch_is_true_or_end(&mut receiver).await;

        fut.await;
    });
}
