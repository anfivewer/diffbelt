use crate::collection::CommitGenerationUpdateReader;
use crate::common::reader::ReaderRecord;
use crate::common::GenerationId;
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use crate::http::data::encoded_generation_id::EncodedGenerationIdJsonData;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderDiffFromDefJsonData {
    pub reader_id: String,
    pub collection_name: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderRecordJsonData {
    reader_id: String,
    collection_name: Option<String>,
    generation_id: EncodedGenerationIdJsonData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReaderJsonData {
    reader_id: String,
    generation_id: EncodedGenerationIdJsonData,
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
            generation_id: EncodedGenerationIdJsonData::encode(
                GenerationId::from_opt_owned(&generation_id).unwrap_or(GenerationId::empty()),
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

impl UpdateReaderJsonData {
    pub fn decode_vec(
        items: Vec<UpdateReaderJsonData>,
        decoder: &StringDecoder,
    ) -> Result<Vec<CommitGenerationUpdateReader>, HttpError> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let UpdateReaderJsonData {
                reader_id,
                generation_id,
            } = item;

            let generation_id = generation_id.into_generation_id()?;

            result.push(CommitGenerationUpdateReader {
                reader_id,
                generation_id,
            });
        }

        Ok(result)
    }
}
