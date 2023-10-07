use crate::common::{IsByteArray, OwnedPhantomId};
use crate::http::errors::HttpError;
use crate::http::util::encoding::StringDecoder;

use crate::util::str_serialization::StrSerializationType;
use diffbelt_types::common::phantom_id::EncodedPhantomIdJsonData;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

pub trait EncodedPhantomIdJsonDataTrait: Sized {
    fn new(phantom_id: OwnedPhantomId, encoding: StrSerializationType) -> Self;
    fn decode(self, decoder: &StringDecoder) -> Result<OwnedPhantomId, HttpError>;
    fn decode_opt(
        value: Option<Self>,
        decoder: &StringDecoder,
    ) -> Result<Option<OwnedPhantomId>, HttpError>;
}

impl EncodedPhantomIdJsonDataTrait for EncodedPhantomIdJsonData {
    fn new(phantom_id: OwnedPhantomId, encoding: StrSerializationType) -> Self {
        let (value, encoding) = encoding.serialize_with_priority(phantom_id.get_byte_array());

        Self {
            value,
            encoding: encoding.to_optional_string(),
        }
    }

    fn decode(self, decoder: &StringDecoder) -> Result<OwnedPhantomId, HttpError> {
        decoder.decode_field_with_map(
            "phantomId",
            self.value,
            "phantomIdEncoding",
            self.encoding,
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

    fn decode_opt(
        value: Option<Self>,
        decoder: &StringDecoder,
    ) -> Result<Option<OwnedPhantomId>, HttpError> {
        let Some(value) = value else {
            return Ok(None);
        };

        let phantom_id = value.decode(&decoder)?;
        Ok(Some(phantom_id))
    }
}
