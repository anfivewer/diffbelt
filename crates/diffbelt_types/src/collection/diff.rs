use crate::common::generation_id::EncodedGenerationIdJsonData;
use crate::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReaderDiffFromDefJsonData {
    pub reader_name: String,
    pub collection_name: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueDiffJsonData {
    pub key: EncodedKeyJsonData,

    pub from_value: Option<Option<EncodedValueJsonData>>,
    pub intermediate_values: Vec<Option<EncodedValueJsonData>>,
    pub to_value: Option<Option<EncodedValueJsonData>>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiffCollectionRequestJsonData {
    pub from_generation_id: Option<EncodedGenerationIdJsonData>,
    pub to_generation_id: Option<EncodedGenerationIdJsonData>,

    pub from_reader: Option<ReaderDiffFromDefJsonData>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiffCollectionResponseJsonData {
    pub from_generation_id: EncodedGenerationIdJsonData,
    pub to_generation_id: EncodedGenerationIdJsonData,
    pub items: Vec<KeyValueDiffJsonData>,
    pub cursor_id: Option<Box<str>>,
}
