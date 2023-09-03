use crate::errors::ConfigParsingError;

use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateMapInstruction {
    pub var: String,
    pub key: String,
    pub value: String,
}

impl UpdateMapInstruction {
    fn from_yaml(yaml: &YamlNode) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}
