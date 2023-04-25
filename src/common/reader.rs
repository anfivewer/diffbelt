use crate::common::OwnedGenerationId;

#[derive(Eq, PartialEq, Debug)]
pub struct ReaderRecord {
    pub reader_name: String,
    pub collection_name: Option<String>,
    pub generation_id: Option<OwnedGenerationId>,
}

#[derive(Clone)]
pub struct ReaderDef {
    pub collection_name: Option<String>,
    pub reader_name: String,
}

pub struct ReaderState {
    pub collection_name: Option<String>,
    pub generation_id: Option<OwnedGenerationId>,
}
