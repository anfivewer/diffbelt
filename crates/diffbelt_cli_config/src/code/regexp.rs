use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_seq, expect_str};
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::YamlNode;

#[derive(Debug)]
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
        let map = expect_map(yaml)?;

        let mut instruction_var = None;
        let mut instruction_regexp = None;
        let mut instruction_groups = None;

        for (key, value) in map {
            let key = expect_str(key)?;

            match key {
                "var" => {
                    instruction_var = Some(expect_str(value)?);
                }
                "regexp" => {
                    instruction_regexp = Some(expect_str(value)?);
                }
                "groups" => {
                    let seq = expect_seq(value)?;
                    let mut groups = Vec::new();

                    for value in seq {
                        let value = expect_str(value)?;
                        groups.push(value.to_string());
                    }

                    instruction_groups = Some(groups);
                }
                _ => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: format!("Unknown regexp prop: \"{}\"", key),
                        position: Some(value.into()),
                    }));
                }
            }
        }

        let (Some(var), Some(regexp), Some(groups)) =
            (instruction_var, instruction_regexp, instruction_groups)
        else {
            return Err(ConfigParsingError::Custom(ExpectedError {
                message: "regexp should have var, regexp, groups".to_string(),
                position: Some(yaml.into()),
            }));
        };

        Ok(Self {
            var: var.to_string(),
            regexp: regexp.to_string(),
            groups,
        })
    }
}
