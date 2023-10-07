use crate::collection::CommitGenerationUpdateReader;
use crate::common::reader::ReaderRecord;
use crate::common::GenerationId;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_encode, encoded_generation_id_data_into_generation_id,
    EncodedGenerationIdJsonData,
};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;
use crate::util::str_serialization::StrSerializationType;
pub use diffbelt_types::collection::diff::ReaderDiffFromDefJsonData;
use diffbelt_types::common::reader::UpdateReaderJsonData;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderRecordJsonData {
    reader_name: String,
    collection_name: Option<String>,
    generation_id: EncodedGenerationIdJsonData,
}

impl From<ReaderRecord> for ReaderRecordJsonData {
    fn from(kv: ReaderRecord) -> Self {
        let ReaderRecord {
            reader_name,
            collection_name,
            generation_id,
        } = kv;

        Self {
            reader_name,
            collection_name,
            generation_id: encoded_generation_id_data_encode(
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

pub trait UpdateReaderJsonDataTrait {
    fn decode_vec(
        items: Vec<UpdateReaderJsonData>,
        _decoder: &StringDecoder,
    ) -> Result<Vec<CommitGenerationUpdateReader>, HttpError>;
}

impl UpdateReaderJsonDataTrait for UpdateReaderJsonData {
    fn decode_vec(
        items: Vec<UpdateReaderJsonData>,
        _decoder: &StringDecoder,
    ) -> Result<Vec<CommitGenerationUpdateReader>, HttpError> {
        let mut result = Vec::with_capacity(items.len());

        for item in items {
            let UpdateReaderJsonData {
                reader_name,
                generation_id,
            } = item;

            let generation_id = encoded_generation_id_data_into_generation_id(generation_id)?;

            result.push(CommitGenerationUpdateReader {
                reader_name: reader_name.into(),
                generation_id,
            });
        }

        Ok(result)
    }
}
