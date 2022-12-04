use std::collections::BTreeSet;
use crate::common::{GenerationId, PhantomId, CollectionKey};

struct PhantomKey {
    key: CollectionKey,
    generation_id: GenerationId,
}

enum CollectionPhantomKeys {
    Sealed(Vec<PhantomKey>),
    InProgress(BTreeSet<PhantomKey>),
}

struct CollectionPhantom {
    id: PhantomId,
    keys: CollectionPhantomKeys,
}
