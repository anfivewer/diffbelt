use crate::util::tokio_runtime::create_single_thread_tokio_runtime;
use std::future::Future;

use std::thread;
use tokio::task::JoinError;

pub fn spawn(f: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(f);
}

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

pub async fn spawn_async_thread<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
    #[cfg(feature = "debug_prints")] name: &str,
) -> tokio::task::JoinHandle<Option<T>> {
    #[cfg(feature = "debug_prints")]
    let name = {
        std::io::stderr()
            .write(format!("Run: {}\n", name).as_bytes())
            .unwrap();
        std::io::stderr().flush().unwrap();

        Box::from(name) as Box<str>
    };

    let join_handle = thread::spawn(move || {
        let runtime = create_single_thread_tokio_runtime().expect("Cannot create tokio runtime");

        runtime.block_on(f)
    });

    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || join_handle.join()).await;

        #[cfg(feature = "debug_prints")]
        {
            std::io::stderr()
                .write(format!("Finish: {}\n", name).as_bytes())
                .unwrap();
            std::io::stderr().flush().unwrap();
        }

        match result {
            Ok(Ok(result)) => Some(result),
            Ok(Err(_)) => None,
            Err(_) => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::common::NeverEq;
    use crate::util::tokio::spawn_async_thread;
    use crate::util::tokio_runtime::create_main_tokio_runtime;

    use std::time::Duration;
    use tokio::sync::{oneshot, watch};
    use tokio::time::sleep;

    #[test]
    fn channels_between_threads() {
        let runtime = create_main_tokio_runtime().unwrap();

        let result = runtime.block_on(async {
            let (result_sender, result_receiver) = oneshot::channel();

            let (sender, receiver) = oneshot::channel::<(usize, oneshot::Sender<usize>)>();

            let a = spawn_async_thread(
                async move {
                    let (answer, sender) = receiver.await.unwrap();
                    sender.send(answer).unwrap_or(());
                },
                #[cfg(feature = "debug_prints")]
                "a",
            )
            .await;

            let b = spawn_async_thread(
                async move {
                    let (new_sender, receiver) = oneshot::channel();

                    sender.send((42, new_sender)).unwrap_or(());

                    let answer = receiver.await.unwrap();

                    result_sender.send(format!("Answer is {}", answer)).unwrap();
                },
                #[cfg(feature = "debug_prints")]
                "b",
            )
            .await;

            a.await.unwrap_or(None);
            b.await.unwrap_or(None);

            result_receiver.await.unwrap()
        });

        assert_eq!(result.as_str(), "Answer is 42");
    }

    #[test]
    fn oneshot_with_watcher() {
        let runtime = create_main_tokio_runtime().unwrap();

        let _result = runtime.block_on(async {
            let (watcher_sender, mut watcher_receiver) = watch::channel(NeverEq);
            let (sender, receiver) = oneshot::channel::<()>();

            let a = spawn_async_thread(
                async move {
                    receiver.await.unwrap();

                    let mut counter = 0;

                    watcher_receiver.borrow_and_update();

                    loop {
                        watcher_receiver.changed().await.unwrap();

                        counter += 1;

                        if counter >= 2 {
                            return;
                        }
                    }
                },
                #[cfg(feature = "debug_prints")]
                "a",
            )
            .await;

            sleep(Duration::from_millis(10)).await;
            sender.send(()).unwrap();

            sleep(Duration::from_millis(1000)).await;
            watcher_sender.send(NeverEq).expect("1");

            sleep(Duration::from_millis(1000)).await;
            watcher_sender.send(NeverEq).expect("2");

            a.await.unwrap_or(None);
        });
    }
}
