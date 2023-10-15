use crate::code::regexp::RegexpInstructionBody;
use crate::errors::ConfigPositionMark;
use crate::interpreter::cleanups::{Cleanups, CompileTimeCleanup};
use crate::interpreter::error::{add_position, InterpreterError};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::value::Value;
use crate::interpreter::var::VarDef;
use diffbelt_util::option::lift_result_from_option;

#[derive(Debug, Clone)]
pub struct RegexpStatement {
    pub regexp: VarPointer,
    pub regexp_mark: ConfigPositionMark,
    pub var: VarPointer,
    pub var_mark: ConfigPositionMark,
    pub groups: Vec<VarPointer>,
    pub start_pos: VarPointer,
    pub start_pos_is_exact: bool,
    pub jump_statement_index_if_not_matches: Option<usize>,
    pub last_index_output: Option<VarPointer>,
    pub rest: Option<VarPointer>,
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
            if_not_matches,
        } = regexp;

        let mut cleanups = Cleanups::new();

        let var_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);
        () = self
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

        let mut groups_ptrs = Vec::with_capacity(groups.len());

        for name in groups {
            let ptr = self.named_var_or_create(name)?;
            groups_ptrs.push(ptr);
        }

        if let Some(regexp) = regexp {
            let regexp_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);
            self.process_expression(&regexp.value, regexp_ptr.clone())
                .map_err(add_position(&regexp.mark))?;

            let regexp_statement_index = self.statements.len();
            self.statements.push(Statement::Regexp(RegexpStatement {
                regexp: regexp_ptr,
                regexp_mark: regexp.mark.clone(),
                var: var_ptr,
                var_mark: var.mark.clone(),
                groups: groups_ptrs,
                start_pos: VarPointer::LiteralUsize(0),
                start_pos_is_exact: false,
                jump_statement_index_if_not_matches: None,
                last_index_output: None,
                rest: None,
            }));

            if let Some(if_not_matches) = if_not_matches {
                // if matches, will jump over not-matches code
                let jump_index = self.statements.len();
                self.statements.push(Statement::Jump { statement_index: 0 });

                () = self.process_code(&if_not_matches.value)?;

                let noop_index = self.statements.len();
                self.statements.push(Statement::Noop);

                let Statement::Jump { statement_index } = &mut self.statements[jump_index] else {
                    panic!("process_regexp:if_not_matches: no jump statement");
                };
                *statement_index = noop_index;

                let Statement::Regexp(regexp_statement) =
                    &mut self.statements[regexp_statement_index]
                else {
                    panic!("process_regexp:if_not_matches: no regexp statement");
                };

                // Skip one instruction to ignore jump
                regexp_statement.jump_statement_index_if_not_matches = Some(jump_index + 1);
            }
        } else if let Some(regexp_multi) = regexp_multi {
            if let Some(if_not_matches) = if_not_matches {
                return Err(InterpreterError::custom_with_mark(
                    "regexp_multi does not supports if_not_matches".to_string(),
                    if_not_matches.mark.clone(),
                ));
            }

            let regexp_ptr = self.temp_var(VarDef::anonymous_string(), &mut cleanups);
            self.process_expression(&regexp_multi.value, regexp_ptr.clone())
                .map_err(add_position(&regexp_multi.mark))?;

            let last_index_ptr = self.temp_var(VarDef::anonymous_u64(), &mut cleanups);
            self.statements.push(Statement::Set {
                value: Value::U64(0),
                destination: last_index_ptr.clone(),
            });

            let rest = rest
                .as_ref()
                .map(|rest| self.named_var_or_create(&rest.value));

            let rest = lift_result_from_option(rest)?;

            let fail_on_non_continuous = fail_on_non_continuous.unwrap_or(false);

            let regexp_statement_index = self.statements.len();
            self.statements.push(Statement::Regexp(RegexpStatement {
                regexp: regexp_ptr,
                regexp_mark: regexp_multi.mark.clone(),
                var: var_ptr.clone(),
                var_mark: var.mark.clone(),
                groups: groups_ptrs,
                start_pos: last_index_ptr.clone(),
                start_pos_is_exact: fail_on_non_continuous,
                jump_statement_index_if_not_matches: None,
                last_index_output: Some(last_index_ptr),
                rest,
            }));

            if let Some(loop_code) = loop_code {
                self.process_code(loop_code)?;
            }

            self.statements.push(Statement::Jump {
                statement_index: regexp_statement_index,
            });

            let last_statement_index = self.statements.len();
            self.statements.push(Statement::Noop);

            let Statement::Regexp(regexp_statement) = &mut self.statements[regexp_statement_index]
            else {
                panic!("process_regexp:regexp_multi: no regexp statement");
            };
            regexp_statement.jump_statement_index_if_not_matches = Some(last_statement_index);
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
