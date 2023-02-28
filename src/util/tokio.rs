use crate::util::global_tokio_runtime::get_global_tokio_runtime_or_panic;
use std::future::Future;
use tokio::task::JoinError;

pub fn spawn(f: impl Future<Output = ()> + Send + 'static) {
    let runtime = get_global_tokio_runtime_or_panic();

    runtime.spawn(f);
}

pub async fn spawn_blocking_async<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> Result<T, JoinError> {
    tokio::task::spawn_blocking(move || {
        let runtime = get_global_tokio_runtime_or_panic();

        runtime.block_on(f)
    })
    .await
}

pub async fn spawn_async_thread<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> tokio::task::JoinHandle<T> {
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let runtime = get_global_tokio_runtime_or_panic();

            runtime.block_on(f)
        })
        .await;

        match result {
            Ok(result) => result,
            Err(err) => {
                panic!("spawn_async_thread JoinError: {:?}", err);
            }
        }
    })
}
