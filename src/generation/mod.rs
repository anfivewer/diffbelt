use crate::common::{CollectionKey, GenerationId};
use std::collections::BTreeSet;
use std::sync::RwLock;

pub enum CollectionGenerationKeys {
    Sealed(Vec<CollectionKey>),
    // Use std::sync::RwLock instead of tokio, this set will be not blocked for a long time
    InProgress(RwLock<BTreeSet<CollectionKey>>),
}

pub struct CollectionGeneration {
    pub id: GenerationId,
    pub keys: CollectionGenerationKeys,
}
