use crate::common::GenerationId;
use crate::generation::CollectionGeneration;
use crate::raw_db::RawDb;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::RwLock;

mod methods;

type ReaderCollectionId<'a> = &'a str;
type ReaderId<'a> = &'a str;

pub struct Collection {
    raw_db: Arc<RawDb>,
    is_manual: bool,
    generation_id: RwLock<RefCell<GenerationId>>,
    // None if this is manual collection and generation is not yet started
    // in non-manual collections always present
    next_generation: RwLock<RefCell<Option<CollectionGeneration>>>,
    get_reader_generation_id: Box<dyn Fn(ReaderCollectionId<'_>, ReaderId<'_>) -> GenerationId>,
}
