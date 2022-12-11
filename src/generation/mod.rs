use crate::common::{CollectionKey, GenerationId};
use std::collections::BTreeSet;

use tokio::sync::watch::Receiver;

#[derive(Clone, Debug)]
pub enum CollectionGenerationKeyProgress {
    Pending,
    AlreadyExists(GenerationId),
    WasPut(GenerationId),
    Err,
}

// TODO: move this enum to collection module and rename it
pub enum CollectionGenerationKeyStatus {
    InProgress(Receiver<CollectionGenerationKeyProgress>),
}
