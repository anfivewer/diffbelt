use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_str};
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateMapInstruction {
    pub var: String,
    pub key: String,
    pub value: String,
}

impl FromYaml for UpdateMapInstruction {
    fn from_yaml(
        _state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}
