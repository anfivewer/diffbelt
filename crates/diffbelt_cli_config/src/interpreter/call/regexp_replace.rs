use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::statement::vars::RegexpReplaceStatement;
use crate::interpreter::var::Var;
use std::ops::Deref;
use std::rc::Rc;

impl<'a> FunctionExecution<'a> {
    pub fn execute_regexp_replace(
        &mut self,
        statement: &RegexpReplaceStatement,
    ) -> Result<(), InterpreterError> {
        let RegexpReplaceStatement { ptr, regexp, to } = statement;

        let input = self.read_var_as_rc_str(ptr, None)?;
        let input_str = input.deref();

        let output = regexp.replace_all(input_str, to.deref());
        let output_str = output.deref();

        if input_str == output_str {
            self.statement_index += 1;
            return Ok(());
        }

        let var = Var::new_string(Rc::from(output_str));
        self.set_var(ptr, var)?;

        self.statement_index += 1;
        Ok(())
    }
}
