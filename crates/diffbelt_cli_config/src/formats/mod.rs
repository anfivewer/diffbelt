use std::ops::Deref;
use std::rc::Rc;

use serde_json::{Error as JsonError, Value as JsonValue};

use crate::formats::json::{json_value_to_var, value_to_json_value};
use crate::interpreter::value::Value;
use crate::interpreter::var::Var;
use crate::CollectionValueFormat;

mod json;

#[derive(Debug)]
pub enum ValueFormatError {
    NotUft8,
    Json(JsonError),
    JsonNotF64Impossible,
    BytesTargetAreNotSupported,
    ValueCannotBeRepresentedAsUtf8(Value),
    BadF64(f64),
    CycleDetected,
}

impl CollectionValueFormat {
    pub fn boxed_bytes_to_var(&self, bytes: Box<[u8]>) -> Result<Var, ValueFormatError> {
        match self {
            CollectionValueFormat::Bytes => {
                let bytes = Rc::<[u8]>::from(bytes);

                Ok(Var::new_bytes(bytes))
            }
            CollectionValueFormat::Utf8 => {
                let bytes = bytes.into_vec();
                let s = String::from_utf8(bytes).map_err(|_| ValueFormatError::NotUft8)?;
                let s = Rc::<str>::from(s);

                Ok(Var::new_string(s))
            }
            CollectionValueFormat::Json => {
                let data = serde_json::from_slice::<JsonValue>(bytes.deref())
                    .map_err(ValueFormatError::Json)?;

                Ok(json_value_to_var(data)?)
            }
        }
    }

    pub fn value_to_boxed_bytes(&self, value: Value) -> Result<Box<[u8]>, ValueFormatError> {
        let bytes = match self {
            CollectionValueFormat::Bytes => {
                return Err(ValueFormatError::BytesTargetAreNotSupported);
            }
            CollectionValueFormat::Utf8 => match value {
                Value::String(s) => Box::from(s.as_bytes()),
                _ => {
                    return Err(ValueFormatError::ValueCannotBeRepresentedAsUtf8(value));
                }
            },
            CollectionValueFormat::Json => {
                let data = value_to_json_value(&value)?;

                let bytes = serde_json::to_vec(&data).map_err(ValueFormatError::Json)?;

                bytes.into_boxed_slice()
            }
        };

        Ok(bytes)
    }
}
