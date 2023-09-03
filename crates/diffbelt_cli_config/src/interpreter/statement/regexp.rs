use crate::code::regexp::RegexpInstructionBody;
use crate::interpreter::error::{add_position_to_no_such_variable, InterpreterError};
use crate::interpreter::expression::{VarPointer, NO_TEMP_VARS};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::VarDef;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegexpStatement {
    pub regexp: VarPointer,
    pub var: VarPointer,
}

impl<'a> FunctionInitState<'a> {
    pub fn process_regexp(
        &mut self,
        regexp: &RegexpInstructionBody,
    ) -> Result<(), InterpreterError> {
        let RegexpInstructionBody {
            var,
            parts,
            regexp,
            groups,
        } = regexp;

        let mut cleanup_statements = Vec::new();

        let var = self
            .process_expression(var.value.as_str(), &mut cleanup_statements)
            .map_err(add_position_to_no_such_variable(&var.mark))?;

        if let Some(parts) = parts {
            for (name, value) in parts {
                let part = self.process_expression(value.as_str(), &mut cleanup_statements)?;

                let index = self.temp_var(VarDef::anonymous_string());
                cleanup_statements.push(Statement::FreeTempVar(index));

                let temp_var_ptr = VarPointer::VarIndex(index);
                self.add_named_var(name.as_str(), temp_var_ptr.clone());

                self.statements.push(Statement::Copy {
                    source: part,
                    destination: temp_var_ptr,
                });
            }
        }

        let regexp = self.process_expression(regexp.as_str(), &mut cleanup_statements)?;

        self.statements
            .push(Statement::Regexp(RegexpStatement { regexp, var }));

        self.push_statements(cleanup_statements);

        Ok(())
    }
}
