use std::future::Future;
use tokio::sync::oneshot;

pub struct AutoSenderOnDrop<T> {
    value_and_sender: Option<(T, oneshot::Sender<T>)>,
}

impl<T> AutoSenderOnDrop<T> {
    pub fn new(value: T) -> (Self, impl Future<Output = T>) {
        let (sender, receiver) = oneshot::channel();

        (
            Self {
                value_and_sender: Some((value, sender)),
            },
            async move { receiver.await.unwrap() },
        )
    }

    pub fn value(&self) -> &T {
        unsafe { &self.value_and_sender.as_ref().unwrap_unchecked().0 }
    }

    pub fn value_mut(&mut self) -> &mut T {
        unsafe { &mut self.value_and_sender.as_mut().unwrap_unchecked().0 }
    }
}

impl<T> Drop for AutoSenderOnDrop<T> {
    fn drop(&mut self) {
        if let Some((value, sender)) = self.value_and_sender.take() {
            sender.send(value).unwrap_or(());
        }
    }
}
