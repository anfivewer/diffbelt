use crate::errors::ConfigParsingError;
use std::rc::Rc;

use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateMapInstruction {
    pub var: String,
    pub key: String,
    pub value: String,
}

impl UpdateMapInstruction {
    fn from_yaml(yaml: &Rc<YamlNode>) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}
