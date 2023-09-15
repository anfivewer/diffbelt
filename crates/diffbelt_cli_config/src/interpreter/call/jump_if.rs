use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::statement::jump::{Condition, JumpIfStatement};

impl<'a> FunctionExecution<'a> {
    pub fn execute_jump_if(&mut self, jump_if: &JumpIfStatement) -> Result<(), InterpreterError> {
        let JumpIfStatement {
            condition,
            statement_index,
        } = jump_if;

        let success = match condition {
            Condition::NonEmptyString(ptr) => {
                let s = self.read_var_as_str(ptr, None)?;
                !s.is_empty()
            }
        };

        if success {
            self.statement_index = *statement_index;
        } else {
            self.statement_index += 1;
        }

        Ok(())
    }
}
