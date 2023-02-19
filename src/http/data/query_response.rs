use crate::collection::methods::query::QueryOk;
use crate::http::data::encoded_generation_id::EncodedGenerationIdFlatJsonData;
use crate::http::data::key_value::KeyValueJsonData;
use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponseJsonData {
    #[serde(flatten)]
    generation_id: EncodedGenerationIdFlatJsonData,
    items: Vec<KeyValueJsonData>,
    cursor_id: Option<String>,
}

impl From<QueryOk> for QueryResponseJsonData {
    fn from(data: QueryOk) -> Self {
        let QueryOk {
            generation_id,
            items,
            cursor_id,
        } = data;

        QueryResponseJsonData {
            generation_id: EncodedGenerationIdFlatJsonData::encode(
                generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
            items: items.into_iter().map(|kv| kv.into()).collect(),
            cursor_id,
        }
    }
}
