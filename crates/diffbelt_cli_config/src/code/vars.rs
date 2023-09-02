use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_str};
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::Deserialize;

#[derive(Debug)]
pub struct VarsInstruction {
    pub vars: Vec<Var>,
}

#[derive(Debug)]
pub struct Var {
    pub name: String,
    pub value: VarProcessing,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VarProcessing {
    ByString(String),
    DateFromUnixMs(DateFromUnixMsProcessing),
}

#[derive(Debug, Deserialize)]
pub struct DateFromUnixMsProcessing {
    date_from_unix_ms: String,
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
        Ok(decode_yaml(yaml)?)
    }
}
