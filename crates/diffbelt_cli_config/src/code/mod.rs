pub mod regexp;
pub mod update_map;
pub mod vars;

use crate::code::regexp::RegexpInstruction;
use crate::code::update_map::UpdateMapInstruction;
use crate::code::vars::VarsInstruction;
use std::collections::HashMap;
use std::rc::Rc;

use crate::errors::WithMark;
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::de::Error;
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

        let first_key = raw
            .as_mapping()
            .and_then(|mapping| mapping.items.first())
            .and_then(|(key, _)| key.as_str());

        if let Some(first_key) = first_key {
            match first_key {
                "regexp" => {
                    let value = decode_yaml(&raw).map_err(|err| D::Error::custom(err))?;
                    return Ok(Instruction::Regexp(value));
                }
                _ => {}
            }
        }

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
    pub value: ReturnValue,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReturnValue {
    Var(WithMark<Rc<str>>),
    Mapping(HashMap<Rc<str>, WithMark<Rc<str>>>),
    Unknown(Rc<YamlNode>),
}

impl<'de> Deserialize<'de> for ReturnValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = YamlNode::deserialize(deserializer)?;

        if let Ok(value) = decode_yaml(&raw) {
            return Ok(ReturnValue::Var(value));
        }
        if let Ok(value) = decode_yaml(&raw) {
            return Ok(ReturnValue::Mapping(value));
        }

        Ok(ReturnValue::Unknown(raw))
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Instruction, ReturnInstruction, ReturnValue};
    use crate::errors::ConfigPositionMark;
    use diffbelt_yaml::serde::WithMark;
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
                    value: ReturnValue::Var(WithMark {
                        value: Rc::from("42"),
                        mark: ConfigPositionMark::empty()
                    })
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
                value: ReturnValue::Var(WithMark {
                    value: Rc::from("42"),
                    mark: ConfigPositionMark::empty()
                })
            })
        )
    }
}
