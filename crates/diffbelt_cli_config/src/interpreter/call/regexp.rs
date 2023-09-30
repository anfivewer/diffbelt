use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;

use crate::interpreter::expression::VarPointer;
use crate::interpreter::statement::regexp::RegexpStatement;
use crate::interpreter::var::Var;
use diffbelt_util::debug_print::debug_print;
use regex::Regex;
use std::ops::Deref;
use std::rc::Rc;

impl<'a> FunctionExecution<'a> {
    pub fn execute_regexp(&mut self, regexp: &RegexpStatement) -> Result<(), InterpreterError> {
        let RegexpStatement {
            regexp,
            regexp_mark,
            var,
            var_mark,
            groups,
            start_pos,
            start_pos_is_exact,
            jump_statement_index_if_not_matches,
            last_index_output,
            rest,
        } = regexp;

        let input = self.read_var_as_rc_str(var, Some(var_mark))?;
        let input = input.deref();

        let regexp = self.read_var_as_rc_str(regexp, Some(regexp_mark))?;
        let regexp = regexp.deref();

        let regexp = Regex::new(regexp).map_err(|_| {
            InterpreterError::custom_with_mark("Invalid regexp".to_string(), regexp_mark.clone())
        })?;

        let start_pos = self.read_var_as_usize(start_pos, None)?;

        let input_slice = &input[start_pos..];

        if input_slice.is_empty() {
            if let Some(rest) = rest {
                self.set_var(rest, Var::new_string(Rc::from("")))?;
            }

            if let Some(index) = jump_statement_index_if_not_matches {
                self.statement_index = *index;
                return Ok(());
            }

            return Err(InterpreterError::custom_with_mark(
                format!("Regexp not matched: \"{input_slice}\""),
                regexp_mark.clone(),
            ));
        }

        let Some(captures) = regexp.captures_at(input, start_pos) else {
            if let Some(rest) = rest {
                self.set_var(rest, Var::new_string(Rc::from(input)))?;

                if let Some(index) = jump_statement_index_if_not_matches {
                    self.statement_index = *index;
                }

                return Ok(());
            }

            return Err(InterpreterError::custom_with_mark(
                format!("Regexp not matched: \"{}\"", input),
                regexp_mark.clone(),
            ));
        };

        let full_match = captures.get(0).unwrap();

        if *start_pos_is_exact {
            let actual_start = full_match.start();
            if actual_start != start_pos {
                return Err(InterpreterError::custom_with_mark(
                    format!("Is not exact match: \"{input}\", /{regexp}/, expected pos {start_pos}, actual {actual_start}"),
                    regexp_mark.clone(),
                ));
            }
        }

        if let Some(last_index_ptr) = last_index_output {
            self.set_var(last_index_ptr, Var::new_u64(full_match.end() as u64))?;
        }

        let count = captures.len();

        if groups.len() != count - 1 {
            return Err(InterpreterError::custom_with_mark(
                format!(
                    "Regexp groups count invalid, expected {} got {}",
                    groups.len(),
                    count - 1
                ),
                regexp_mark.clone(),
            ));
        }

        for (i, destination) in groups.iter().enumerate() {
            let value = captures.get(i + 1).map(|m| m.as_str()).unwrap_or("");

            let value = Var::new_string(Rc::from(value));

            self.set_var(destination, value)?;
        }

        self.statement_index += 1;

        Ok(())
    }
}
