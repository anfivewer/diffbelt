use crate::common::{CollectionKey, GenerationId, PhantomId};
use std::collections::BTreeSet;

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
