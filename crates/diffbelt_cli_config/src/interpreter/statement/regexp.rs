use crate::code::regexp::RegexpInstructionBody;
use crate::interpreter::error::{add_position, InterpreterError};
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

        let mut cleanups = Vec::new();
        let mut drop_names = Vec::new();

        let var_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);

        let _: () = self
            .process_expression(var.value.as_str(), var_ptr.clone(), &mut cleanups)
            .map_err(add_position(&var.mark))?;

        if let Some(parts) = parts {
            for (name, value) in parts {
                let part_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);

                let _: () =
                    self.process_expression(value.as_str(), part_ptr.clone(), &mut cleanups)?;

                self.add_named_var(name.as_str(), part_ptr.clone());
                drop_names.push(name.as_str());
            }
        }

        let regexp_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);
        let _: () = self.process_expression(regexp.as_str(), regexp_ptr.clone(), &mut cleanups)?;

        self.statements.push(Statement::Regexp(RegexpStatement {
            regexp: regexp_ptr,
            var: var_ptr,
        }));

        self.push_statements(cleanups);
        for name in drop_names {
            self.drop_named_var(name)?;
        }

        Ok(())
    }
}
