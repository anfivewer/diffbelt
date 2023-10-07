use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedPhantomIdJsonData {
    pub value: String,
    pub encoding: Option<String>,
}
