pub mod template_str;

use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{ExpectError, InterpreterError};
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
        destination: VarPointer,
        cleanups: &mut Cleanups,
    ) -> Result<(), InterpreterError> {
        if SIMPLE_VAR.is_match(expr) {
            let ptr = self.named_var(expr)?;
            self.statements.push(Statement::Copy {
                source: ptr,
                destination,
            });
            return Ok(());
        }
        if let Some(captures) = LITERAL_STR.captures(expr) {
            let s = &captures[1];
            self.statements.push(Statement::Copy {
                source: VarPointer::LiteralStr(Rc::from(s)),
                destination,
            });
            return Ok(());
        }
        if let Some(captures) = TEMPLATE_STR.captures(expr) {
            let s = &captures[1];
            return self.process_template_str(s, destination);
        }

        Err(InterpreterError::InvalidExpression(expr.to_string()))
    }
}
