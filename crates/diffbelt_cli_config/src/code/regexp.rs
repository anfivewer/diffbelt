use crate::errors::{ConfigPositionMark, WithMark};
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct RegexpInstruction {
    pub regexp: RegexpInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct RegexpInstructionBody {
    pub var: WithMark<String>,
    pub parts: Option<IndexMap<String, String>>,
    pub regexp: String,
    pub groups: Vec<String>,
}
