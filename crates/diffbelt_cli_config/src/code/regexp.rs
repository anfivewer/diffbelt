use crate::errors::ConfigParsingError;
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegexpInstruction {
    pub var: String,
    pub regexp: String,
    pub groups: Vec<String>,
}

impl FromYaml for RegexpInstruction {
    fn from_yaml(
        _state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}
