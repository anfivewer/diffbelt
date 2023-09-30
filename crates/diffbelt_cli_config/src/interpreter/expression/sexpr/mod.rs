use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use diffbelt_util::Wrap;

pub enum SExpr {
    EmptyMap,
    EmptyList,
}

impl SExpr {
    pub fn parse(s: &str) -> Result<Self, InterpreterError> {
        let as_invalid_expr_lexpr = |_| InterpreterError::InvalidExpression(s.to_string());
        let as_invalid_expr_opt = || InterpreterError::InvalidExpression(s.to_string());

        let expr = lexpr::from_str_custom(s, lexpr::parse::Options::new())
            .map_err(as_invalid_expr_lexpr)?;

        let expr = expr.as_cons().ok_or_else(as_invalid_expr_opt)?;
        let (name, params) = expr.as_pair();

        let name = name.as_symbol().ok_or_else(as_invalid_expr_opt)?;

        match name {
            "map" => {
                params.as_null().ok_or_else(as_invalid_expr_opt)?;
                Ok(SExpr::EmptyMap)
            }
            "list" => {
                params.as_null().ok_or_else(as_invalid_expr_opt)?;
                Ok(SExpr::EmptyList)
            }
            unknown => Err(InterpreterError::InvalidExpression(format!(
                "Unknown s-expr fn {}",
                unknown
            ))),
        }
    }
}

impl<'a> FunctionInitState<'a> {
    pub fn process_s_expr(
        &mut self,
        expr: SExpr,
        destination: VarPointer,
    ) -> Result<(), InterpreterError> {
        match expr {
            SExpr::EmptyMap => self.statements.push(Statement::Set {
                value: Value::Map(Wrap::wrap(HashMap::new())),
                destination,
            }),
            SExpr::EmptyList => self.statements.push(Statement::Set {
                value: Value::List(Wrap::wrap(Vec::new())),
                destination,
            }),
        }

        Ok(())
    }
}
