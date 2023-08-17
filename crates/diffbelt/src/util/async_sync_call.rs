use std::future::Future;
use tokio::sync::oneshot;

pub async fn async_sync_call<T, Fut: Future<Output = ()>, F: FnOnce(oneshot::Sender<T>) -> Fut>(
    fun: F,
) -> Result<T, oneshot::error::RecvError> {
    let (sender, receiver) = oneshot::channel();

    fun(sender).await;

    receiver.await
}
