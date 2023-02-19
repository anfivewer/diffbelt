use crate::collection::methods::diff::DiffOk;

use crate::http::data::encoded_generation_id::{
    EncodedGenerationIdFlatJsonData, EncodedNullableGenerationIdFlatJsonData,
};

use crate::http::data::key_value_diff::KeyValueDiffJsonData;
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResponseJsonData {
    #[serde(flatten)]
    from_generation_id: EncodedNullableGenerationIdFlatJsonData,
    #[serde(flatten)]
    generation_id: EncodedGenerationIdFlatJsonData,
    items: Vec<KeyValueDiffJsonData>,
    cursor_id: Option<String>,
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
            from_generation_id: EncodedNullableGenerationIdFlatJsonData::encode(
                from_generation_id.as_ref().map(|x| x.as_ref()),
                StrSerializationType::Utf8,
            ),
            generation_id: EncodedGenerationIdFlatJsonData::encode(
                to_generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
            items: KeyValueDiffJsonData::encode_vec(items),
            cursor_id,
        }
    }
}
