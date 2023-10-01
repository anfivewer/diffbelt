use crate::YamlMark;
use serde::de::StdError;

use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum YamlDecodingError {
    Custom(ExpectError),
}

#[derive(Debug)]
pub struct ExpectError {
    pub message: String,
    pub position: Option<YamlMark>,
}

impl StdError for YamlDecodingError {}

fn write_message_and_position(
    f: &mut Formatter<'_>,
    message: &String,
    position: &Option<YamlMark>,
) -> std::fmt::Result {
    f.write_str(message.as_str())?;

    if let Some(position) = position {
        f.write_fmt(format_args!(" at {}:{}", position.line, position.column))?;
    }

    Ok(())
}

impl Display for YamlDecodingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlDecodingError::Custom(ExpectError { message, position }) => {
                write_message_and_position(f, message, position)?;
            }
        }

        Ok(())
    }
}

impl serde::de::Error for YamlDecodingError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        YamlDecodingError::Custom(ExpectError {
            message: msg.to_string(),
            position: None,
        })
    }
}
