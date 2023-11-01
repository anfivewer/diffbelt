use std::str::from_utf8;
#[macro_export]
macro_rules! value_encoding_into_bytes {
    ( $original:ident ) => {
        impl $original {
            pub fn into_bytes(self) -> Result<Box<[u8]>, crate::errors::IntoBytesError<$original>> {
                let Some(encoding) = &self.encoding else {
                    return Ok(self.value.into_bytes().into_boxed_slice());
                };

                match encoding.as_str() {
                    "base64" => {
                        let bytes = base64::decode(&self.value)
                            .map_err(|_| crate::errors::IntoBytesError::Base64(self))?;

                        Ok(bytes.into_boxed_slice())
                    }
                    _ => Err(crate::errors::IntoBytesError::UnknownEncoding(self)),
                }
            }

            pub fn from_bytes_slice(bytes: &[u8]) -> Self {
                match ::std::str::from_utf8(bytes) {
                    Ok(value) => Self {
                        value: value.to_string(),
                        encoding: None,
                    },
                    Err(_) => {
                        let value = base64::encode(bytes);

                        Self {
                            value,
                            encoding: Some(String::from("base64")),
                        }
                    }
                }
            }

            pub fn from_boxed_bytes(bytes: Box<[u8]>) -> Self {
                match String::from_utf8(bytes.into_vec()) {
                    Ok(value) => Self {
                        value,
                        encoding: None,
                    },
                    Err(err) => {
                        let value = base64::encode(err.into_bytes());

                        Self {
                            value,
                            encoding: Some(String::from("base64")),
                        }
                    }
                }
            }
        }
    };
}
