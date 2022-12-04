use std::collections::BTreeSet;
use crate::common::{GenerationId, CollectionKey};

enum CollectionGenerationKeys {
    Sealed(Vec<CollectionKey>),
    InProgress(BTreeSet<CollectionKey>),
}

struct CollectionGeneration {
    id: GenerationId,
    keys: CollectionGenerationKeys,
}
