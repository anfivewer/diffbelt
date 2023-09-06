use crate::code;
use crate::code::regexp::RegexpInstructionBody;
use crate::code::vars::{
    DateFromUnixMsProcessing, NonEmptyStringProcessing, VarProcessing, VarsInstruction,
};
use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{add_position, ExpectError, InterpreterError};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::jump::Condition;
use crate::interpreter::statement::Statement;

#[derive(Debug, Clone)]
pub struct VarsStatement {
    //
}

impl<'a> FunctionInitState<'a> {
    pub fn process_vars_instruction(
        &mut self,
        vars: &VarsInstruction,
    ) -> Result<(), InterpreterError> {
        let VarsInstruction { vars } = vars;

        let mut cleanups = Cleanups::new();

        for var in vars {
            let code::vars::Var { name, value } = var;

            let var_ptr = self.named_var_or_create(name)?;

            match value {
                VarProcessing::ByString(expr) => {
                    self.process_expression(&expr.value, var_ptr, &mut cleanups)
                        .map_err(add_position(&expr.mark))?;
                }
                VarProcessing::DateFromUnixMs(date_from_unix_ms) => {
                    let DateFromUnixMsProcessing {
                        date_from_unix_ms: expr,
                    } = date_from_unix_ms;

                    self.process_expression(&expr.value, var_ptr.clone(), &mut cleanups)
                        .map_err(add_position(&expr.mark))?;
                    self.statements
                        .push(Statement::DateFromUnixMs { ptr: var_ptr });
                }
                VarProcessing::NonEmptyString(non_empty_string) => {
                    let NonEmptyStringProcessing { non_empty_string } = non_empty_string;

                    let mut jumps_to_end = Vec::new();

                    for expr in non_empty_string {
                        self.process_expression(&expr.value, var_ptr.clone(), &mut cleanups)
                            .map_err(add_position(&expr.mark))?;

                        jumps_to_end.push(self.jump_if(Condition::NonEmptyString(var_ptr.clone())));
                    }

                    let noop_index = self.statements.len();
                    self.statements.push(Statement::Noop);

                    for update_jump_to in jumps_to_end {
                        update_jump_to(self, noop_index);
                    }
                }
                VarProcessing::Unknown(node) => {
                    return Err(InterpreterError::Custom(ExpectError {
                        message: "unknown var processing".to_string(),
                        position: Some(node.into()),
                    }))
                }
            }
        }

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
