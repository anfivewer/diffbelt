pub mod regexp;
pub mod update_map;
pub mod vars;

use crate::code::regexp::RegexpInstruction;
use crate::code::update_map::UpdateMapInstruction;
use crate::code::vars::VarsInstruction;
use crate::errors::ConfigParsingError;
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Code {
    instructions: Vec<Instruction>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum Instruction {
    Vars(VarsInstruction),
    UpdateMap(UpdateMapInstruction),
    Regexp(RegexpInstruction),
    Return(ReturnInstruction),
    Unknown(YamlNode),
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ReturnInstruction {
    #[serde(rename = "return")]
    pub value: String,
}

impl Code {
    pub fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}

impl Instruction {
    pub fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Instruction, ReturnInstruction};
    use diffbelt_yaml::{decode_yaml, parse_yaml};

    #[test]
    fn single_return_test() {
        let input = r#"
- return: 42
"#;

        let input = &parse_yaml(input).expect("parsing")[0];
        let value: Code = decode_yaml(input).expect("decode");

        println!("code {:?}", value);
    }

    #[test]
    fn return_instruction_test() {
        let input = "return: 42";

        let input = &parse_yaml(input).expect("parsing")[0];
        let value: Instruction = decode_yaml(input).expect("decode");

        assert_eq!(
            value,
            Instruction::Return(ReturnInstruction {
                value: "42".to_string()
            })
        )
    }
}
