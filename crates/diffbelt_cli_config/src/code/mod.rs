pub mod regexp;
pub mod update_map;
pub mod vars;

use crate::code::regexp::RegexpInstruction;
use crate::code::update_map::UpdateMapInstruction;
use crate::code::vars::VarsInstruction;
use std::rc::Rc;

use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct Code {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Instruction {
    Vars(VarsInstruction),
    UpdateMap(UpdateMapInstruction),
    Regexp(RegexpInstruction),
    Return(ReturnInstruction),
    Unknown(Rc<YamlNode>),
}

impl<'de> Deserialize<'de> for Instruction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = YamlNode::deserialize(deserializer)?;

        if let Ok(value) = decode_yaml(&raw) {
            return Ok(Instruction::Vars(value));
        }
        if let Ok(value) = decode_yaml(&raw) {
            return Ok(Instruction::UpdateMap(value));
        }
        if let Ok(value) = decode_yaml(&raw) {
            return Ok(Instruction::Regexp(value));
        }
        if let Ok(value) = decode_yaml(&raw) {
            return Ok(Instruction::Return(value));
        }

        Ok(Instruction::Unknown(raw))
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ReturnInstruction {
    #[serde(rename = "return")]
    pub value: String,
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Instruction, ReturnInstruction};
    use diffbelt_yaml::{decode_yaml, parse_yaml};
    use std::rc::Rc;

    #[test]
    fn single_return_test() {
        let input = r#"
- return: 42
"#;

        let input = parse_yaml(input)
            .expect("parsing")
            .into_iter()
            .next()
            .expect("no doc");
        let input = Rc::new(input);
        let value: Code = decode_yaml(&input).expect("decode");

        assert_eq!(
            value,
            Code {
                instructions: vec![Instruction::Return(ReturnInstruction {
                    value: "42".to_string()
                })]
            }
        )
    }

    #[test]
    fn return_instruction_test() {
        let input = "return: 42";

        let input = parse_yaml(input)
            .expect("parsing")
            .into_iter()
            .next()
            .expect("no doc");
        let input = Rc::new(input);
        let value: Instruction = decode_yaml(&input).expect("decode");

        assert_eq!(
            value,
            Instruction::Return(ReturnInstruction {
                value: "42".to_string()
            })
        )
    }
}
