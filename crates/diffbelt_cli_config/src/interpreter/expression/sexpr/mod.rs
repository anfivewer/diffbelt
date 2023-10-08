use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::value::Value;

use diffbelt_util::Wrap;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

pub enum SExpr {
    None,
    EmptyMap,
    EmptyList,
    IsNone { var_name: Rc<str> },
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
            "none" => {
                () = params.as_null().ok_or_else(as_invalid_expr_opt)?;
                Ok(SExpr::None)
            }
            "map" => {
                () = params.as_null().ok_or_else(as_invalid_expr_opt)?;
                Ok(SExpr::EmptyMap)
            }
            "list" => {
                () = params.as_null().ok_or_else(as_invalid_expr_opt)?;
                Ok(SExpr::EmptyList)
            }
            "is_none" => {
                let cons = params.as_cons().ok_or_else(|| {
                    InterpreterError::InvalidExpression(
                        "is_none should have variable name".to_string(),
                    )
                })?;

                let (value, params) = cons.as_pair();
                () = params.as_null().ok_or_else(|| {
                    InterpreterError::InvalidExpression(
                        "is_none should have only variable name".to_string(),
                    )
                })?;

                let value = value.as_symbol().ok_or_else(|| {
                    InterpreterError::InvalidExpression(
                        "is_none variable name is not a string".to_string(),
                    )
                })?;

                Ok(SExpr::IsNone {
                    var_name: Rc::from(value),
                })
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
            SExpr::None => {
                self.statements.push(Statement::Set {
                    value: Value::None,
                    destination,
                });
            }
            SExpr::IsNone { var_name } => {
                let ptr = self.named_var(var_name.deref())?;

                self.statements.push(Statement::IsNone { ptr, destination });
            }
            SExpr::EmptyMap => self.statements.push(Statement::SetDyn {
                value: create_new_map,
                destination,
            }),
            SExpr::EmptyList => self.statements.push(Statement::SetDyn {
                value: create_new_list,
                destination,
            }),
        }

        Ok(())
    }
}

fn create_new_map() -> Value {
    Value::Map(Wrap::wrap(HashMap::new()))
}

fn create_new_list() -> Value {
    Value::List(Wrap::wrap(Vec::new()))
}
