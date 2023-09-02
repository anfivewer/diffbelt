use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_str};
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::YamlNode;

#[derive(Debug)]
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
        let map = expect_map(yaml)?;

        let mut instruction_var = None;
        let mut instruction_key = None;
        let mut instruction_value = None;

        for (key, value) in map {
            let key = expect_str(key)?;

            match key {
                "var" => {
                    instruction_var = Some(expect_str(value)?);
                }
                "key" => {
                    instruction_key = Some(expect_str(value)?);
                }
                "value" => {
                    instruction_value = Some(expect_str(value)?);
                }
                _ => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: format!("Unknown update_map prop: \"{}\"", key),
                        position: Some(value.into()),
                    }));
                }
            }
        }

        let (Some(var), Some(key), Some(value)) =
            (instruction_var, instruction_key, instruction_value)
        else {
            return Err(ConfigParsingError::Custom(ExpectedError {
                message: "update_map should have var, key, value".to_string(),
                position: Some(yaml.into()),
            }));
        };

        Ok(Self {
            var: var.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}
