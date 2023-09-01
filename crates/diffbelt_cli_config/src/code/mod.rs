mod vars;

use crate::code::vars::VarProcessing;
use crate::errors::{ConfigParsingError, ExpectedError};
use crate::util::expect::{expect_map, expect_seq, expect_str};
use crate::YamlParsingState;
use diffbelt_yaml::YamlNode;

#[derive(Debug)]
pub struct Code {
    pub instructions: Vec<Instruction>,
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

#[derive(Debug)]
pub enum Instruction {
    Regexp(RegexpInstruction),
    Vars(VarsInstruction),
    Return(String),
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
                    let instruction = VarsInstruction::from_yaml(state, value)?;
                    result = Some((key, Instruction::Vars(instruction)));
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

#[derive(Debug)]
pub struct RegexpInstruction {
    //
}

#[derive(Debug)]
pub struct VarsInstruction {
    pub vars: Vec<Var>,
}

#[derive(Debug)]
pub struct Var {
    pub name: String,
    pub value: VarProcessing,
}

impl VarsInstruction {
    pub fn from_yaml(
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
