use crate::value_encoding_into_bytes;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncodedGenerationIdJsonData {
    pub value: String,
    pub encoding: Option<String>,
}

impl PartialEq<Self> for EncodedGenerationIdJsonData {
    fn eq(&self, other: &Self) -> bool {
        if self.encoding == other.encoding {
            return self.value == other.value;
        }

        let self_encoding = self.encoding.as_ref().map(|x| x.as_str()).unwrap_or("utf8");
        let other_encoding = self.encoding.as_ref().map(|x| x.as_str()).unwrap_or("utf8");

        if self_encoding == other_encoding {
            return self.value == other.value;
        }

        let mut maybe_self_bytes = None;
        let mut maybe_other_bytes = None;

        fn to_bytes<'a>(
            encoding: &str,
            s: &'a String,
            maybe_bytes: &'a mut Option<Vec<u8>>,
        ) -> Option<&'a [u8]> {
            if encoding == "utf8" {
                Some(s.as_bytes())
            } else if encoding == "base64" {
                let Ok(bytes) = base64::decode(s.as_str()) else {
                    return None;
                };

                *maybe_bytes = Some(bytes);

                maybe_bytes.as_ref().map(|x| x.as_slice())
            } else {
                None
            }
        }

        let Some(self_value) = to_bytes(self_encoding, &self.value, &mut maybe_self_bytes) else {
            return false;
        };
        let Some(other_value) = to_bytes(other_encoding, &other.value, &mut maybe_other_bytes)
        else {
            return false;
        };

        self_value == other_value
    }
}

impl Eq for EncodedGenerationIdJsonData {}

impl EncodedGenerationIdJsonData {
    pub fn new_str(value: String) -> Self {
        Self {
            value,
            encoding: None,
        }
    }
}

value_encoding_into_bytes!(EncodedGenerationIdJsonData);
