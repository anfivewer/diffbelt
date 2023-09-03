use crate::interpreter::error::{ExpectError, InterpreterError};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;

impl<'a> FunctionInitState<'a> {
    pub fn push_statements(&mut self, mut statements: Vec<Statement>) {
        self.statements.append(&mut statements);
    }

    pub fn add_named_var(&mut self, name: &str, var: VarPointer) {
        self.named_vars
            .entry(name.to_string())
            .and_modify(|vars| {
                vars.push(var.clone());
            })
            .or_insert_with(|| vec![var]);
    }

    pub fn drop_named_var(&mut self, name: &str) -> Result<(), InterpreterError> {
        let vars = self.named_vars.get_mut(name).ok_or_else(|| {
            InterpreterError::Custom(ExpectError {
                message: "drop_named_var: no such var".to_string(),
                position: None,
            })
        })?;

        let _: VarPointer = vars.pop().ok_or_else(|| {
            InterpreterError::Custom(ExpectError {
                message: "drop_named_var: no such var".to_string(),
                position: None,
            })
        })?;

        Ok(())
    }

    pub fn named_var(&self, name: &str) -> Result<VarPointer, InterpreterError> {
        let vars = self
            .named_vars
            .get(name)
            .ok_or_else(|| InterpreterError::NoSuchVariable(name.to_string()))?;

        let ptr = vars
            .last()
            .ok_or_else(|| InterpreterError::NoSuchVariable(name.to_string()))?;

        Ok(ptr.clone())
    }
}
