use crate::common::{IsByteArray, OwnedPhantomId};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;

use crate::util::str_serialization::StrSerializationType;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedPhantomIdFlatJsonData {
    phantom_id: String,
    phantom_id_encoding: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedOptionalPhantomIdFlatJsonData {
    phantom_id: Option<String>,
    phantom_id_encoding: Option<String>,
}

impl EncodedPhantomIdFlatJsonData {
    pub fn new(phantom_id: OwnedPhantomId, encoding: StrSerializationType) -> Self {
        let (phantom_id, encoding) = encoding.serialize_with_priority(phantom_id.get_byte_array());

        Self {
            phantom_id,
            phantom_id_encoding: encoding.to_optional_string(),
        }
    }

    pub fn decode(self, decoder: &StringDecoder) -> Result<OwnedPhantomId, HttpError> {
        decoder.decode_field_with_map(
            "phantomId",
            self.phantom_id,
            "phantomIdEncoding",
            self.phantom_id_encoding,
            |bytes| {
                if bytes.is_empty() {
                    return Err(HttpError::Generic400(
                        "invalid phantomId, it cannot be empty",
                    ));
                }

                OwnedPhantomId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid phantomId, length should be <= 255",
                )))
            },
        )
    }
}

impl EncodedOptionalPhantomIdFlatJsonData {
    pub fn decode(self, decoder: &StringDecoder) -> Result<Option<OwnedPhantomId>, HttpError> {
        decoder.decode_opt_field_with_map(
            "phantomId",
            self.phantom_id,
            "phantomIdEncoding",
            self.phantom_id_encoding,
            |bytes| {
                if bytes.is_empty() {
                    return Err(HttpError::Generic400(
                        "invalid phantomId, it cannot be empty",
                    ));
                }

                OwnedPhantomId::from_boxed_slice(bytes).or(Err(HttpError::Generic400(
                    "invalid phantomId, length should be <= 255",
                )))
            },
        )
    }
}
