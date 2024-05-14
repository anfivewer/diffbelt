use crate::http::errors::HttpError;
use crate::util::str_serialization::StrSerializationType;

pub struct StringDecoder {
    default_type: StrSerializationType,
}

impl StringDecoder {
    pub fn new(default_encoding: StrSerializationType) -> Self {
        Self {
            default_type: default_encoding,
        }
    }

    pub fn decode_field_with_map<R, F: FnOnce(Box<[u8]>) -> Result<R, HttpError>>(
        &self,
        field_name: &str,
        value: String,
        field_encoding_name: &str,
        field_encoding: Option<String>,
        map: F,
    ) -> Result<R, HttpError> {
        let t = opt_string_into_encoding_with_default(
            field_encoding_name,
            field_encoding,
            self.default_type,
        )?;

        let value = into_decoded_value(field_name, value, t)?;

        map(value)
    }
}

fn opt_string_into_encoding_with_default(
    field_name: &str,
    encoding: Option<String>,
    default_encoding: StrSerializationType,
) -> Result<StrSerializationType, HttpError> {
    let result = match encoding {
        Some(encoding) => StrSerializationType::from_str(&encoding),
        None => {
            return Ok(default_encoding);
        }
    };

    result.or(Err(HttpError::GenericString400(
        format_encoding_type_parsing_err(field_name),
    )))
}

fn into_decoded_value(
    field_name: &str,
    value: String,
    encoding: StrSerializationType,
) -> Result<Box<[u8]>, HttpError> {
    encoding
        .deserialize(&value)
        .map_err(|_| HttpError::GenericString400(format!("invalid {}, check encoding", field_name)))
}

fn format_encoding_type_parsing_err(field_name: &str) -> String {
    format!(
        "invalid {}, allowed \"base64\" or default (\"utf8\")",
        field_name
    )
}
