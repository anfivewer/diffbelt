use std::sync::Arc;
use crate::raw_db::RawDb;
use crate::common::GenerationId;

mod methods;

type ReaderCollectionId<'a> = &'a str;
type ReaderId<'a> = &'a str;

pub struct Collection {
    raw_db: Arc<RawDb>,
    get_reader_generation_id: Box<dyn Fn(ReaderCollectionId<'_>, ReaderId<'_>) -> GenerationId>,
}
