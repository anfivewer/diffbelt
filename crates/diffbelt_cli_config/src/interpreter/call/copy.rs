use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::var::Var;
use diffbelt_util_no_std::cast::usize_to_u64;

impl<'a> FunctionExecution<'a> {
    pub fn execute_copy(
        &mut self,
        source: &VarPointer,
        destination: &VarPointer,
    ) -> Result<(), InterpreterError> {
        let source = match source {
            VarPointer::VarIndex(index) => self
                .vars
                .get(*index)
                .ok_or_else(|| {
                    InterpreterError::custom_without_mark(format!(
                        "FunctionExecution: no source at index {}",
                        index
                    ))
                })?
                .clone(),
            VarPointer::LiteralStr(s) => Var::new_string(s.clone()),
            VarPointer::LiteralUsize(n) => Var::new_u64(usize_to_u64(*n)),
        };

        self.set_var(destination, source)?;

        self.statement_index += 1;

        Ok(())
    }
}
