use crate::requests::DiffbeltRequests;
use diffbelt_http_client::client::DiffbeltClient;
use std::sync::Arc;
use tokio::select;
use tokio::sync::oneshot;

pub struct DiffbeltRequestsClientImpl {
    stop_sender: Option<oneshot::Sender<()>>,
}

impl DiffbeltRequestsClientImpl {
    pub fn new(client: Arc<DiffbeltClient>) -> (DiffbeltRequests, Self) {
        let (requests, mut receiver) = DiffbeltRequests::new();
        let (stop_sender, mut stop_receiver) = oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                let request = select! {
                    request = receiver.recv() => {
                        request
                    },
                    _ = &mut stop_receiver => {
                        return;
                    },
                };

                let Some((request, sender)) = request else {
                    return;
                };

                let client_inner = client.clone();
                tokio::spawn(async move {
                    let result = client_inner.flatbuffers_raw_call(request).await;

                    let () = sender.send(result).unwrap_or(());
                });
            }
        });

        (
            requests,
            Self {
                stop_sender: Some(stop_sender),
            },
        )
    }
}

impl Drop for DiffbeltRequestsClientImpl {
    fn drop(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            sender.send(()).unwrap_or(());
        }
    }
}
