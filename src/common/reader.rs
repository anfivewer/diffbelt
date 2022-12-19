use crate::common::OwnedGenerationId;

#[derive(Eq, PartialEq, Debug)]
pub struct ReaderRecord {
    pub reader_id: String,
    pub collection_id: Option<String>,
    pub generation_id: Option<OwnedGenerationId>,
}
