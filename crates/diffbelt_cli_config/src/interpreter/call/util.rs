use crate::errors::ConfigPositionMark;
use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::interpreter::value::{PrimitiveValue, Value};
use crate::interpreter::var::Var;
use diffbelt_util::cast::{u64_to_usize, usize_to_u64};
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
            VarPointer::LiteralUsize(_) => {
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

    pub fn read_var_value(&self, ptr: &VarPointer) -> Result<Value, InterpreterError> {
        let source = match ptr {
            VarPointer::VarIndex(index) => self.read_var_by_index(*index)?,
            VarPointer::LiteralStr(s) => {
                return Ok(Value::String(s.clone()));
            }
            VarPointer::LiteralUsize(value) => {
                return Ok(Value::U64(usize_to_u64(*value)));
            }
        };

        source
            .value
            .as_ref()
            .map(|holder| holder.value.clone())
            .ok_or_else(|| InterpreterError::custom_without_mark("Uninitialized value".to_string()))
    }

    pub fn read_var_as_str<'b>(
        &'b self,
        ptr: &'b VarPointer,
        mark: Option<&ConfigPositionMark>,
    ) -> Result<&'b str, InterpreterError> {
        let source = match ptr {
            VarPointer::VarIndex(index) => self.read_var_by_index(*index)?,
            VarPointer::LiteralStr(s) => {
                return Ok(s.deref());
            }
            VarPointer::LiteralUsize(_) => {
                return Err(InterpreterError::custom(
                    "Value is not a string".to_string(),
                    mark.map(|x| x.clone()),
                ));
            }
        };

        let value = source.as_str().ok_or_else(|| {
            InterpreterError::custom("Value is not a string".to_string(), mark.map(|x| x.clone()))
        })?;

        Ok(value)
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
            VarPointer::LiteralUsize(_) => {
                return Err(InterpreterError::custom(
                    "Value is not a string".to_string(),
                    mark.map(|x| x.clone()),
                ));
            }
        };

        let value = source.as_rc_str().ok_or_else(|| {
            InterpreterError::custom("Value is not a string".to_string(), mark.map(|x| x.clone()))
        })?;

        Ok(value)
    }

    pub fn read_var_as_usize<'b>(
        &'b self,
        ptr: &'b VarPointer,
        mark: Option<&ConfigPositionMark>,
    ) -> Result<usize, InterpreterError> {
        let source = match ptr {
            VarPointer::VarIndex(index) => self.read_var_by_index(*index)?,
            VarPointer::LiteralStr(_s) => {
                return Err(InterpreterError::custom(
                    "Value is a string".to_string(),
                    mark.map(|x| x.clone()),
                ));
            }
            VarPointer::LiteralUsize(value) => {
                return Ok(*value);
            }
        };

        let value = source.value.as_ref().ok_or_else(|| {
            InterpreterError::custom("Uninitialized value".to_string(), mark.map(|x| x.clone()))
        })?;

        match &value.value {
            Value::U64(value) => Ok(u64_to_usize(*value)),
            _ => Err(InterpreterError::custom(
                "Value is not a number".to_string(),
                mark.map(|x| x.clone()),
            )),
        }
    }

    pub fn read_var_as_map<'b>(
        &'b self,
        ptr: &'b VarPointer,
        mark: Option<&ConfigPositionMark>,
    ) -> Result<&'b RefCell<HashMap<PrimitiveValue, Value>>, InterpreterError> {
        let source = match ptr {
            VarPointer::VarIndex(index) => self.read_var_by_index(*index)?,
            _ => {
                return Err(InterpreterError::custom(
                    "Expected pointer".to_string(),
                    mark.map(|x| x.clone()),
                ));
            }
        };

        let value = source.as_map().ok_or_else(|| {
            InterpreterError::custom("Value is not a map".to_string(), mark.map(|x| x.clone()))
        })?;

        Ok(value)
    }

    pub fn read_var_as_list<'b>(
        &'b self,
        ptr: &'b VarPointer,
        mark: Option<&ConfigPositionMark>,
    ) -> Result<&'b RefCell<Vec<Value>>, InterpreterError> {
        let source = match ptr {
            VarPointer::VarIndex(index) => self.read_var_by_index(*index)?,
            _ => {
                return Err(InterpreterError::custom(
                    "Expected pointer".to_string(),
                    mark.map(|x| x.clone()),
                ));
            }
        };

        let value = source.as_list().ok_or_else(|| {
            InterpreterError::custom("Value is not a list".to_string(), mark.map(|x| x.clone()))
        })?;

        Ok(value)
    }
}

pub fn var_as_str(var: &Var) -> Result<&str, InterpreterError> {
    var.as_str().ok_or_else(|| {
        InterpreterError::custom_without_mark("variable is not a string".to_string())
    })
}
