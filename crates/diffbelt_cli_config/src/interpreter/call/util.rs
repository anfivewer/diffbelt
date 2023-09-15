use crate::errors::ConfigPositionMark;
use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::value::Value;
use crate::interpreter::var::Var;
use std::ops::Deref;
use std::rc::Rc;

impl<'a> FunctionExecution<'a> {
    pub fn borrow_var_by_index(&self, index: usize) -> Result<&Var, InterpreterError> {
        self.vars.get(index).ok_or_else(|| {
            InterpreterError::custom_without_mark(format!("no var at index {}", index))
        })
    }

    pub fn set_var(
        &mut self,
        destination: &VarPointer,
        value: Var,
    ) -> Result<(), InterpreterError> {
        let destination = match destination {
            VarPointer::VarIndex(index) => self.vars.get_mut(*index).ok_or_else(|| {
                InterpreterError::custom_without_mark(
                    "FunctionExecution:set_value_to_var: no destination".to_string(),
                )
            })?,
            VarPointer::LiteralStr(_) => {
                return Err(InterpreterError::custom_without_mark(
                    "FunctionExecution: destination cannot be literal".to_string(),
                ))
            }
        };

        *destination = value;

        Ok(())
    }

    pub fn read_var_by_index(&self, index: usize) -> Result<&Var, InterpreterError> {
        self.vars.get(index).ok_or_else(|| {
            InterpreterError::custom_without_mark(format!(
                "FunctionExecution: no source at index {}",
                index
            ))
        })
    }

    pub fn read_var_as_rc_str(
        &self,
        ptr: &VarPointer,
        mark: Option<&ConfigPositionMark>,
    ) -> Result<Rc<str>, InterpreterError> {
        let source = match ptr {
            VarPointer::VarIndex(index) => self.read_var_by_index(*index)?,
            VarPointer::LiteralStr(s) => {
                return Ok(s.clone());
            }
        };

        let value = source.as_rc_str().ok_or_else(|| {
            InterpreterError::custom("Value is not a string".to_string(), mark.map(|x| x.clone()))
        })?;

        Ok(value)
    }
}

pub fn var_as_str(var: &Var) -> Result<&str, InterpreterError> {
    var.as_str().ok_or_else(|| {
        InterpreterError::custom_without_mark("variable is not a string".to_string())
    })
}
