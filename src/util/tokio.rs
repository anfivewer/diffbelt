use crate::TOKIO_RUNTIME;
use std::future::Future;
use tokio::task::JoinError;

pub async fn spawn_blocking_async<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> Result<T, JoinError> {
    tokio::task::spawn_blocking(move || {
        let runtime = unsafe {
            match &TOKIO_RUNTIME {
                Some(runtime) => runtime.clone(),
                None => {
                    panic!("no tokio runtime");
                }
            }
        };

        runtime.block_on(f)
    })
    .await
}

pub fn spawn_async_thread<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> std::thread::JoinHandle<T> {
    std::thread::spawn(move || {
        let runtime = unsafe {
            match &TOKIO_RUNTIME {
                Some(runtime) => runtime.clone(),
                None => {
                    panic!("no tokio runtime");
                }
            }
        };

        runtime.block_on(f)
    })
}
