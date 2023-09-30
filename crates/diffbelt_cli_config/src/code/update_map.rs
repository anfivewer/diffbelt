use std::rc::Rc;
use serde::Deserialize;
use crate::errors::WithMark;

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
