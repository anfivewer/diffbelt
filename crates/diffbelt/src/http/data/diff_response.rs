use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use crate::collection::methods::diff::DiffOk;

use crate::common::GenerationId;
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_encode,
};
use crate::http::data::key_value_diff::{KeyValueDiffJsonData, KeyValueDiffJsonDataTrait};
use crate::util::str_serialization::StrSerializationType;

impl From<DiffOk> for DiffCollectionResponseJsonData {
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
