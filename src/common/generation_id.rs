use crate::common::reader::ReaderDef;
use crate::common::OwnedGenerationId;

pub enum GenerationIdSource {
    Value(Option<OwnedGenerationId>),
    Reader(ReaderDef),
}
