pub mod template_str;

use crate::interpreter::error::InterpreterError;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use regex::Regex;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum VarPointer {
    VarIndex(usize),
    LiteralStr(Rc<str>),
}

pub const NO_TEMP_VARS: fn(&str) -> Option<VarPointer> = |_| None;

lazy_static::lazy_static! {
    static ref SIMPLE_VAR: Regex = Regex::new("^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    static ref LITERAL_STR: Regex = Regex::new("^'(.*)'$").unwrap();
    static ref TEMPLATE_STR: Regex = Regex::new("^\"(.*)\"$").unwrap();
}

impl<'a> FunctionInitState<'a> {
    pub fn process_expression(
        &mut self,
        expr: &str,
        cleanups_holder: &mut Vec<Statement>,
    ) -> Result<VarPointer, InterpreterError> {
        if SIMPLE_VAR.is_match(expr) {
            return self.named_var(expr);
        }
        if let Some(captures) = LITERAL_STR.captures(expr) {
            let s = &captures[1];
            return Ok(VarPointer::LiteralStr(Rc::from(s)));
        }
        if let Some(captures) = TEMPLATE_STR.captures(expr) {
            let s = &captures[1];
            return self.process_template_str(s);
        }

        println!("process expr: {}", expr);

        todo!()
    }
}
