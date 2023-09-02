use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_str};
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::YamlNode;

#[derive(Debug)]
pub struct VarsInstruction {
    pub vars: Vec<Var>,
}

#[derive(Debug)]
pub struct Var {
    pub name: String,
    pub value: VarProcessing,
}

#[derive(Debug)]
pub enum VarProcessing {
    ByString(String),
    DateFromUnixMs(String),
}

impl FromYaml for VarsInstruction {
    fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        let map = expect_map(yaml)?;

        let mut vars = Vec::new();

        for (name, value) in &map.items {
            let name = expect_str(name)?;
            let value = VarProcessing::from_yaml(state, value)?;

            vars.push(Var {
                name: name.to_string(),
                value,
            });
        }

        return Ok(Self { vars });
    }
}

impl VarProcessing {
    pub fn from_yaml(
        _state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        let s = yaml.as_str();

        if let Some(s) = s {
            return Ok(VarProcessing::ByString(s.to_string()));
        }

        let map = expect_map(yaml)?;

        let mut result: Option<(&str, VarProcessing)> = None;

        for (key, value) in &map.items {
            let key = expect_str(key)?;

            if let Some((prev_key, _)) = &result {
                return Err(ConfigParsingError::UnknownKey(ExpectedError {
                    message: format!("Var transform \"{}\" should not have \"{}\"", prev_key, key),
                    position: Some(yaml.into()),
                }));
            }

            match key {
                "date_from_unix_ms" => {
                    let value = expect_str(value)?;
                    result = Some((key, VarProcessing::DateFromUnixMs(value.to_string())));
                }
                _ => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: format!("Unknown var transform: \"{}\"", key),
                        position: Some(yaml.into()),
                    }));
                }
            }
        }

        let Some((_, result)) = result else {
            return Err(ConfigParsingError::Custom(ExpectedError {
                message: "Noop var?".to_string(),
                position: Some(yaml.into()),
            }));
        };

        Ok(result)
    }
}
