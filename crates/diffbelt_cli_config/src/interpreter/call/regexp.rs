use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;

use crate::interpreter::expression::VarPointer;
use crate::interpreter::statement::regexp::RegexpStatement;
use crate::interpreter::var::Var;
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
            fail_on_non_full_match_if_no_rest_and_start_pos_is_exact,
            last_index_output,
            rest,
        } = regexp;

        if rest.is_some()
            || *fail_on_non_full_match_if_no_rest_and_start_pos_is_exact
            || jump_statement_index_if_not_matches.is_some()
            || *start_pos_is_exact
            || start_pos != &VarPointer::LiteralUsize(0)
            || last_index_output.is_some()
        {
            todo!();
        }

        let input = self.read_var_as_rc_str(var, Some(var_mark))?;
        let input = input.deref();

        let regexp = self.read_var_as_rc_str(regexp, Some(regexp_mark))?;
        let regexp = regexp.deref();

        let regexp = Regex::new(regexp).map_err(|_| {
            InterpreterError::custom_with_mark("Invalid regexp".to_string(), regexp_mark.clone())
        })?;

        let captures = regexp.captures(input).ok_or_else(|| {
            InterpreterError::custom_with_mark(
                format!("Regexp not matched: \"{}\"", input),
                regexp_mark.clone(),
            )
        })?;

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
