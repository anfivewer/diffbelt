pub mod regexp;
pub mod update_map;
pub mod vars;

use crate::code::regexp::RegexpInstruction;
use crate::code::update_map::UpdateMapInstruction;
use crate::code::vars::VarsInstruction;
use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_seq, expect_str};
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::YamlNode;

#[derive(Debug)]
pub struct Code {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Vars(VarsInstruction),
    UpdateMap(UpdateMapInstruction),
    Regexp(RegexpInstruction),
    Return(String),
}

impl Code {
    pub fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        let mut instructions = Vec::new();

        let instructions_seq = expect_seq(yaml)?;

        for instruction in &instructions_seq.items {
            let instruction = Instruction::from_yaml(state, instruction)?;
            instructions.push(instruction);
        }

        Ok(Self { instructions })
    }
}

impl Instruction {
    pub fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        let map = expect_map(yaml)?;

        let mut result: Option<(&str, Instruction)> = None;

        for (key, value) in &map.items {
            let key = expect_str(key)?;

            if let Some((instruction_name, _)) = &result {
                return Err(ConfigParsingError::UnknownKey(ExpectedError {
                    message: format!(
                        "Instruction \"{}\" cannot have \"{}\" prop",
                        *instruction_name, key
                    ),
                    position: Some(yaml.into()),
                }));
            }

            match key {
                "vars" => {
                    let instruction = FromYaml::from_yaml(state, value)?;
                    result = Some((key, Instruction::Vars(instruction)));
                }
                "update_map" => {
                    let instruction = FromYaml::from_yaml(state, value)?;
                    result = Some((key, Instruction::UpdateMap(instruction)));
                }
                "regexp" => {
                    let instruction = FromYaml::from_yaml(state, value)?;
                    result = Some((key, Instruction::Regexp(instruction)));
                }
                "return" => {
                    let value = expect_str(value)?;
                    result = Some((key, Instruction::Return(value.to_string())));
                }
                _ => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: format!("Unknown instruction \"{}\"", key),
                        position: Some((&yaml.start_mark).into()),
                    }));
                }
            }
        }

        let Some((_, instruction)) = result else {
            return Err(ConfigParsingError::Custom(ExpectedError {
                message: "Noop instruction?".to_string(),
                position: Some((&yaml.start_mark).into()),
            }));
        };

        Ok(instruction)
    }
}
