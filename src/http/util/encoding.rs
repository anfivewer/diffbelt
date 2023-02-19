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

    pub fn from_default_encoding_string(
        default_encoding_field_name: &str,
        encoding: Option<String>,
    ) -> Result<Self, HttpError> {
        let t = opt_string_into_encoding(default_encoding_field_name, encoding)?;

        Ok(Self { default_type: t })
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

    pub fn decode_field_with_map_and_type<R, F: FnOnce(Box<[u8]>) -> Result<R, HttpError>>(
        &self,
        field_name: &str,
        value: String,
        field_encoding_name: &str,
        field_encoding: Option<String>,
        map: F,
    ) -> Result<(R, StrSerializationType), HttpError> {
        let t = opt_string_into_encoding_with_default(
            field_encoding_name,
            field_encoding,
            self.default_type,
        )?;

        let value = into_decoded_value(field_name, value, t)?;

        Ok((map(value)?, t))
    }

    pub fn decode_opt_field_with_map<R, F: FnOnce(Box<[u8]>) -> Result<R, HttpError>>(
        &self,
        field_name: &str,
        value: Option<String>,
        field_encoding_name: &str,
        field_encoding: Option<String>,
        map: F,
    ) -> Result<Option<R>, HttpError> {
        let Some(value) = value else { return Ok(None); };

        let result =
            self.decode_field_with_map(field_name, value, field_encoding_name, field_encoding, map);

        match result {
            Ok(result) => Ok(Some(result)),
            Err(err) => Err(err),
        }
    }

    pub fn decode_opt_field_with_map_and_type<R, F: FnOnce(Box<[u8]>) -> Result<R, HttpError>>(
        &self,
        field_name: &str,
        value: Option<String>,
        field_encoding_name: &str,
        field_encoding: Option<String>,
        map: F,
    ) -> Result<(Option<R>, StrSerializationType), HttpError> {
        let value = match value {
            Some(value) => value,
            None => {
                let t = opt_string_into_encoding_with_default(
                    field_encoding_name,
                    field_encoding,
                    self.default_type,
                )?;

                return Ok((None, t));
            }
        };

        let result = self.decode_field_with_map_and_type(
            field_name,
            value,
            field_encoding_name,
            field_encoding,
            map,
        );

        match result {
            Ok((result, t)) => Ok((Some(result), t)),
            Err(err) => Err(err),
        }
    }
}

fn opt_string_into_encoding(
    field_name: &str,
    encoding: Option<String>,
) -> Result<StrSerializationType, HttpError> {
    let result = StrSerializationType::from_option_string(encoding);

    result.map_err(|_| HttpError::GenericString400(format_encoding_type_parsing_err(field_name)))
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
