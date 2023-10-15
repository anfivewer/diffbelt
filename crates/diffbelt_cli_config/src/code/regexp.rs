use crate::code::Code;
use crate::errors::WithMark;
use indexmap::IndexMap;
use serde::Deserialize;
use std::rc::Rc;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct RegexpInstruction {
    pub regexp: RegexpInstructionBody,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct RegexpInstructionBody {
    pub var: WithMark<Rc<str>>,
    pub parts: Option<IndexMap<Rc<str>, WithMark<Rc<str>>>>,
    pub regexp: Option<WithMark<Rc<str>>>,
    pub regexp_multi: Option<WithMark<Rc<str>>>,
    pub fail_on_non_continuous: Option<bool>,
    pub rest: Option<WithMark<Rc<str>>>,
    pub groups: Vec<Rc<str>>,
    #[serde(rename = "loop")]
    pub loop_code: Option<Code>,
    pub if_not_matches: Option<WithMark<Code>>,
}
