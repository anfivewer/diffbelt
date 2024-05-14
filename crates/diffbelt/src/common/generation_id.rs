use crate::common::reader::ReaderDef;
use crate::common::OwnedGenerationId;

#[derive(Clone)]
pub enum GenerationIdSource {
    Value(Option<OwnedGenerationId>),
    Reader(ReaderDef),
}
