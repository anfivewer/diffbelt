use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use diffbelt_util::serde::deserialize_strict_null::deserialize_strict_null;
use crate::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueUpdateJsonData {
    pub key: EncodedKeyJsonData,
    pub if_not_present: Option<bool>,

    #[serde(deserialize_with = "deserialize_strict_null")]
    pub value: Option<EncodedValueJsonData>,
}
