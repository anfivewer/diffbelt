pub const COLLECTION_CF_GENERATIONS: &str = "gens";
pub const COLLECTION_CF_GENERATIONS_SIZE: &str = "gens_size";
pub const COLLECTION_CF_PHANTOMS: &str = "phantoms";

pub const COLLECTION_CF_META: &str = "meta";
pub const COLLECTION_META_IS_MANUAL_KEY: &[u8] = b"is_manual";
pub const COLLECTION_META_GENERATION_ID_KEY: &[u8] = b"generation_id";
pub const COLLECTION_META_NEXT_GENERATION_ID_KEY: &[u8] = b"next_generation_id";
pub const COLLECTION_META_READER_KEY_PREFIX: &[u8] = b"reader:";
pub const COLLECTION_META_READER_KEY_PREFIX_END: &[u8] = b"reader;";
pub const COLLECTION_META_PREV_PHANTOM_ID_KEY: &[u8] = b"prev_phantom_id";
/** All phantoms less than this can be removed */
pub const COLLECTION_META_PREV_GC_PHANTOM_ID_KEY: &[u8] = b"gc_phantom_id";
pub const COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX: &[u8] = b"phantom:";
pub const COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX_END: &[u8] = b"phantom;";

pub const COLLECTION_GET_KEYS_AROUND_MAX_LIMIT: usize = 1000;
