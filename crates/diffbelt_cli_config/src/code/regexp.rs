use std::rc::Rc;
use crate::errors::{ConfigPositionMark, WithMark};
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct RegexpInstruction {
    pub regexp: RegexpInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct RegexpInstructionBody {
    pub var: WithMark<Rc<str>>,
    pub parts: Option<IndexMap<Rc<str>, WithMark<Rc<str>>>>,
    pub regexp: WithMark<Rc<str>>,
    pub groups: Vec<Rc<str>>,
}
