use crate::util::global_tokio_runtime::get_global_tokio_runtime_or_panic;
use std::future::Future;
use tokio::task::JoinError;

pub async fn spawn_blocking_async<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> Result<T, JoinError> {
    tokio::task::spawn_blocking(move || {
        let runtime = get_global_tokio_runtime_or_panic();

        runtime.block_on(f)
    })
    .await
}

pub fn spawn_async_thread<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> std::thread::JoinHandle<T> {
    std::thread::spawn(move || {
        let runtime = get_global_tokio_runtime_or_panic();

        runtime.block_on(f)
    })
}
