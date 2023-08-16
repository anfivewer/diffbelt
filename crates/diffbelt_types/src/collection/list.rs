use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCollectionsItemJsonData {
    pub name: String,
    pub is_manual: bool,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCollectionsResponseJsonData {
    pub items: Vec<ListCollectionsItemJsonData>,
}
