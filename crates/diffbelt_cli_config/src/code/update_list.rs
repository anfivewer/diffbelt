use crate::errors::WithMark;
use serde::Deserialize;
use std::rc::Rc;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateListInstruction {
    pub update_list: UpdateListInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpdateListInstructionBody {
    pub var: WithMark<Rc<str>>,
    pub push: WithMark<Rc<str>>,
}
