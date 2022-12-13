use tokio::sync::watch::Receiver;

use crate::common::OwnedGenerationId;

#[derive(Clone, Debug)]
pub enum CuncurrentPutStatusProgress {
    Pending,
    AlreadyExists(OwnedGenerationId),
    WasPut(OwnedGenerationId),
    Err,
}

pub enum ConcurrentPutStatus {
    InProgress(Receiver<CuncurrentPutStatusProgress>),
}
