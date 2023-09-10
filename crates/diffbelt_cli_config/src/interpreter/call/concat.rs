use crate::interpreter::call::util::var_as_str;
use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::statement::concat::ConcatStatement;
use crate::interpreter::var::Var;
use std::ops::Deref;
use std::rc::Rc;

impl<'a> FunctionExecution<'a> {
    pub fn execute_concat(&mut self, concat: &ConcatStatement) -> Result<(), InterpreterError> {
        let ConcatStatement { parts, destination } = concat;

        let mut strings = Vec::with_capacity(parts.len());

        for part in parts {
            match part {
                VarPointer::VarIndex(index) => {
                    let var = self.borrow_var_by_index(*index)?;
                    let s = var_as_str(var)?;

                    strings.push(s);
                }
                VarPointer::LiteralStr(s) => {
                    strings.push(s.deref());
                }
            }
        }

        let value = Rc::<str>::from(strings.concat());
        let value = Var::new_string(value);

        self.set_var(destination, value)?;

        self.statement_index += 1;

        Ok(())
    }
}
