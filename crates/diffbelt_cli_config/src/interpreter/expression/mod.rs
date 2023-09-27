pub mod sexpr;
pub mod template_str;

use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::sexpr::SExpr;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use regex::Regex;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum VarPointer {
    VarIndex(usize),
    LiteralStr(Rc<str>),
    LiteralUsize(usize),
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
    ) -> Result<(), InterpreterError> {
        if expr.is_empty() {
            return Err(InterpreterError::InvalidExpression(expr.to_string()));
        }
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
        if let Some(first) = expr.get(0..1) {
            if first == "(" {
                let s_expr = SExpr::parse(expr)?;
                return self.process_s_expr(s_expr, destination);
            }
        }

        Err(InterpreterError::InvalidExpression(expr.to_string()))
    }
}
