use crate::collection::methods::diff::DiffOk;

use crate::common::GenerationId;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_encode, EncodedGenerationIdJsonData,
};
use crate::http::data::key_value_diff::KeyValueDiffJsonData;
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResponseJsonData {
    from_generation_id: EncodedGenerationIdJsonData,
    to_generation_id: EncodedGenerationIdJsonData,
    items: Vec<KeyValueDiffJsonData>,
    cursor_id: Option<Box<str>>,
}

impl From<DiffOk> for DiffResponseJsonData {
    fn from(data: DiffOk) -> Self {
        let DiffOk {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id,
        } = data;

        Self {
            from_generation_id: encoded_generation_id_data_encode(
                GenerationId::from_opt_owned(&from_generation_id).unwrap_or(GenerationId::empty()),
                StrSerializationType::Utf8,
            ),
            to_generation_id: encoded_generation_id_data_encode(
                to_generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
            items: KeyValueDiffJsonData::encode_vec(items),
            cursor_id,
        }
    }
}
