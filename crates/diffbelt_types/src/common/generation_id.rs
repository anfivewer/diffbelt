use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncodedGenerationIdJsonData {
    pub value: String,
    pub encoding: Option<String>,
}

#[derive(Debug)]
pub enum IntoBytesError {
    UnknownEncoding(EncodedGenerationIdJsonData),
    Base64(EncodedGenerationIdJsonData),
}

impl EncodedGenerationIdJsonData {
    pub fn into_bytes(self) -> Result<EncodedGenerationIdJsonDataBytesCow, IntoBytesError> {
        let Some(encoding) = &self.encoding else {
            return Ok(EncodedGenerationIdJsonDataBytesCow::Utf8(self));
        };

        match encoding.as_str() {
            "base64" => {
                let bytes =
                    base64::decode(&self.value).map_err(|_| IntoBytesError::Base64(self))?;

                Ok(EncodedGenerationIdJsonDataBytesCow::Bytes(bytes))
            }
            _ => Err(IntoBytesError::UnknownEncoding(self)),
        }
    }
}

pub enum EncodedGenerationIdJsonDataBytesCow {
    Utf8(EncodedGenerationIdJsonData),
    Bytes(Vec<u8>),
}

impl EncodedGenerationIdJsonDataBytesCow {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            EncodedGenerationIdJsonDataBytesCow::Utf8(generation_id) => {
                generation_id.value.as_bytes()
            }
            EncodedGenerationIdJsonDataBytesCow::Bytes(bytes) => bytes.as_slice(),
        }
    }
}
