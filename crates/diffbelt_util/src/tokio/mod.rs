use crate::tokio_runtime::create_single_thread_tokio_runtime;
use std::future::Future;
use std::thread;
use tokio::task::{JoinError, LocalSet};

pub async fn spawn_blocking_async<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> Result<T, JoinError> {
    let result = tokio::task::spawn_blocking(move || {
        let runtime = create_single_thread_tokio_runtime().expect("Cannot create tokio runtime");

        runtime.block_on(f)
    })
    .await;

    result
}

pub async fn spawn_async_thread_local<
    T: Send + 'static,
    Fut: Future<Output = T> + 'static,
    F: (FnOnce() -> Fut) + Send + 'static,
>(
    f: F,
) -> tokio::task::JoinHandle<Option<T>> {
    let join_handle = thread::spawn(move || {
        let runtime = create_single_thread_tokio_runtime().expect("Cannot create tokio runtime");

        runtime.block_on(async move {
            let local = LocalSet::new();

            local.run_until(f()).await
        })
    });

    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || join_handle.join()).await;

        match result {
            Ok(Ok(result)) => Some(result),
            Ok(Err(_)) => None,
            Err(_) => None,
        }
    })
}
