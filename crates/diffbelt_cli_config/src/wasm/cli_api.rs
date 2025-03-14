use crate::errors::RunTransformError;
use diffbelt_transforms::base::error::TransformError;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct WasmCliApi {
    sender: mpsc::Sender<(String, oneshot::Sender<Result<(), RunTransformError>>)>,
}

impl WasmCliApi {
    pub fn new(
        sender: mpsc::Sender<(String, oneshot::Sender<Result<(), RunTransformError>>)>,
    ) -> Self {
        Self { sender }
    }

    pub async fn run_transform(&self, name: String) -> Result<(), RunTransformError> {
        let (sender, receiver) = oneshot::channel();

        let () = self
            .sender
            .send((name, sender))
            .await
            .map_err(|_| TransformError::Unspecified(String::from("cannot send task")))?;

        let result = receiver
            .await
            .map_err(|_| TransformError::Unspecified(String::from("cannot receive task")))?;

        result
    }
}
