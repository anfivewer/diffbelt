use crate::common::reader::ReaderRecord;
use crate::common::{GenerationId, IsByteArray, KeyValue, OwnedGenerationId};
use crate::http::data::encoded_generation_id::EncodedOptionalGenerationIdFlatJsonData;
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderRecordJsonData {
    reader_id: String,
    collection_name: Option<String>,
    #[serde(flatten)]
    generation_id: EncodedOptionalGenerationIdFlatJsonData,
}

impl From<ReaderRecord> for ReaderRecordJsonData {
    fn from(kv: ReaderRecord) -> Self {
        let ReaderRecord {
            reader_id,
            collection_id,
            generation_id,
        } = kv;

        Self {
            reader_id,
            collection_name: collection_id,
            generation_id: EncodedOptionalGenerationIdFlatJsonData::encode(
                GenerationId::from_opt_owned(&generation_id),
                StrSerializationType::Utf8,
            ),
        }
    }
}

impl ReaderRecordJsonData {
    pub fn encode_vec(items: Vec<ReaderRecord>) -> Vec<Self> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            result.push(Self::from(item));
        }

        result
    }
}
