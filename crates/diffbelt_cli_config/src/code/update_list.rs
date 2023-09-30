use std::rc::Rc;
use serde::Deserialize;
use crate::errors::WithMark;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateListInstruction {
    pub update_list: UpdateListInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateListInstructionBody {
    pub var: WithMark<Rc<str>>,
    pub push: WithMark<Rc<str>>,
}
