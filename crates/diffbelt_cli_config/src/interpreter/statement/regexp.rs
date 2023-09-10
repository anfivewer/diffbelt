use crate::code::regexp::RegexpInstructionBody;
use crate::errors::{ConfigPositionMark, WithMark};
use crate::interpreter::cleanups::{Cleanups, CompileTimeCleanup};
use crate::interpreter::error::{add_position, InterpreterError};
use crate::interpreter::expression::{VarPointer, NO_TEMP_VARS};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::VarDef;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegexpStatement {
    pub regexp: VarPointer,
    pub regexp_mark: ConfigPositionMark,
    pub var: VarPointer,
    pub groups: Vec<VarPointer>,
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
            regexp_multi,
            fail_on_non_continuous,
            rest,
            groups,
            loop_code,
        } = regexp;

        let mut cleanups = Cleanups::new();

        let var_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);

        let _: () = self
            .process_expression(&var.value, var_ptr.clone())
            .map_err(add_position(&var.mark))?;

        if let Some(parts) = parts {
            for (name, value) in parts {
                let part_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);

                self.process_expression(&value.value, part_ptr.clone())
                    .map_err(add_position(&value.mark))?;

                self.add_named_var(name.clone(), part_ptr.clone());
                cleanups
                    .compile_time
                    .push(CompileTimeCleanup::DropNamedVar(name.clone()));
            }
        }

        if let Some(regexp) = regexp {
            let regexp_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);
            self.process_expression(&regexp.value, regexp_ptr.clone())
                .map_err(add_position(&regexp.mark))?;

            let mut groups_ptrs = Vec::with_capacity(groups.len());

            for name in groups {
                let ptr = self.named_var_or_create(name)?;
                groups_ptrs.push(ptr);
            }

            self.statements.push(Statement::Regexp(RegexpStatement {
                regexp: regexp_ptr,
                regexp_mark: regexp.mark.clone(),
                var: var_ptr,
                groups: groups_ptrs,
            }));
        } else if let Some(regexp_multi) = regexp_multi {
            let regexp_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);
            self.process_expression(&regexp_multi.value, regexp_ptr.clone())
                .map_err(add_position(&regexp_multi.mark))?;

            self.statements.push(Statement::Todo("process_regexp:regexp_multi".to_string()));
        } else {
            return Err(InterpreterError::custom_with_mark(
                "regexp should have regexp or regexp_multi field".to_string(),
                var.mark.clone(),
            ));
        }

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
