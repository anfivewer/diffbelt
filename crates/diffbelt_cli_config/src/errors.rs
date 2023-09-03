use diffbelt_yaml::serde::error::YamlDecodingError;
use diffbelt_yaml::serde::Mark;
use diffbelt_yaml::{YamlMark, YamlNode};

#[derive(Debug)]
pub enum ConfigParsingError {
    ExpectedMap(ExpectedError),
    ExpectedSeq(ExpectedError),
    ExpectedString(ExpectedError),
    ExpectedBool(ExpectedError),
    UnknownKey(ExpectedError),
    Custom(ExpectedError),
}

#[derive(Debug)]
pub struct ExpectedError {
    pub message: String,
    pub position: Option<ConfigPositionMark>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConfigPositionMark {
    pub index: u64,
    pub line: u64,
    pub column: u64,
}

pub type WithMark<T> = diffbelt_yaml::serde::WithMark<T, ConfigPositionMark>;

impl Mark for ConfigPositionMark {
    fn new(index: u64, line: u64, column: u64) -> Self {
        Self {
            index,
            line,
            column,
        }
    }
}

impl From<&YamlNode> for ConfigPositionMark {
    fn from(value: &YamlNode) -> Self {
        let YamlMark {
            index,
            line,
            column,
        } = &value.start_mark;

        Self {
            index: *index,
            line: *line,
            column: *column,
        }
    }
}

impl From<&YamlMark> for ConfigPositionMark {
    fn from(value: &YamlMark) -> Self {
        let YamlMark {
            index,
            line,
            column,
        } = value;

        Self {
            index: *index,
            line: *line,
            column: *column,
        }
    }
}

impl From<YamlDecodingError> for ConfigParsingError {
    fn from(value: YamlDecodingError) -> Self {
        match value {
            YamlDecodingError::Custom(error) => {
                let diffbelt_yaml::serde::error::ExpectError { message, position } = error;

                ConfigParsingError::Custom(ExpectedError {
                    message,
                    position: position.map(|x| (&x).into()),
                })
            }
        }
    }
}
