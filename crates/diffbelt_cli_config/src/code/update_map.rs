use crate::errors::WithMark;
use serde::Deserialize;
use std::rc::Rc;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateMapInstruction {
    pub update_map: UpdateMapInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateMapInstructionBody {
    pub var: WithMark<Rc<str>>,
    pub key: WithMark<Rc<str>>,
    pub value: WithMark<Rc<str>>,
}
