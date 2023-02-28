use crate::util::tokio::spawn_async_thread;
use std::future::Future;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

pub struct AsyncTaskThread<T> {
    task_sender: mpsc::Sender<T>,
    stop_sender: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

pub struct TaskPoller<T> {
    task_receiver: mpsc::Receiver<T>,
    stop_receiver: oneshot::Receiver<()>,
}

impl<T> TaskPoller<T> {
    pub async fn poll(&mut self) -> Option<T> {
        tokio::select! {
            result = self.task_receiver.recv() => {
                match result {
                    Some(task) => Some(task),
                    None => {
                        return None;
                    }
                }
            },
            _ = &mut self.stop_receiver => {
                return None;
            },
        }
    }
}

impl<Task> AsyncTaskThread<Task> {
    pub async fn new<
        Data,
        Fut: Future<Output = ()> + Send + 'static,
        F: FnOnce(Data, TaskPoller<Task>) -> Fut,
    >(
        run: F,
        data: Data,
    ) -> Self {
        let (task_sender, task_receiver) = mpsc::channel(1000);
        let (stop_sender, stop_receiver) = oneshot::channel();

        let async_task = run(
            data,
            TaskPoller {
                task_receiver,
                stop_receiver,
            },
        );

        let join_handle = spawn_async_thread(async_task).await;

        Self {
            task_sender,
            stop_sender: Some(stop_sender),
            join_handle: Some(join_handle),
        }
    }

    pub async fn add_task(&self, task: Task) {
        self.task_sender.send(task).await.unwrap_or(());
    }

    #[allow(dead_code)]
    pub async fn stop(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            sender.send(()).unwrap_or(());
        }

        if let Some(join_handle) = self.join_handle.take() {
            join_handle.await.unwrap_or(())
        };
    }
}

impl<T> Drop for AsyncTaskThread<T> {
    fn drop(&mut self) {
        let stop_sender = self.stop_sender.take();

        match stop_sender {
            Some(stop_sender) => {
                stop_sender.send(()).unwrap_or(());
            }
            None => {}
        }
    }
}
