use crate::code::Code;
use crate::errors::WithMark;
use serde::Deserialize;
use std::rc::Rc;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ConditionInstruction {
    #[serde(rename = "if")]
    pub value: ConditionInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ConditionInstructionBody {
    pub condition: WithMark<Rc<str>>,
    pub then: Code,
}
