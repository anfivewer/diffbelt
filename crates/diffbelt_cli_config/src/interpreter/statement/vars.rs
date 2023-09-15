use crate::code;
use crate::code::regexp::RegexpInstructionBody;
use crate::code::vars::{
    DateFromUnixMsProcessing, NonEmptyStringProcessing, ParseDateToMsProcessing,
    ParseUintProcessing, RegexpReplaceProcessing, RegexpReplaceProcessingBody, VarProcessing,
    VarsInstruction,
};
use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{add_position, ExpectError, InterpreterError};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::jump::Condition;
use crate::interpreter::statement::parse_date::ParseDateToMsStatement;
use crate::interpreter::statement::Statement;
use diffbelt_yaml::YamlNodeValue;
use regex::Regex;

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
                    self.process_expression(&expr.value, var_ptr)
                        .map_err(add_position(&expr.mark))?;
                }
                VarProcessing::DateFromUnixMs(date_from_unix_ms) => {
                    let DateFromUnixMsProcessing {
                        date_from_unix_ms: expr,
                    } = date_from_unix_ms;

                    self.process_expression(&expr.value, var_ptr.clone())
                        .map_err(add_position(&expr.mark))?;
                    self.statements
                        .push(Statement::DateFromUnixMs { ptr: var_ptr });
                }
                VarProcessing::NonEmptyString(non_empty_string) => {
                    let NonEmptyStringProcessing { non_empty_string } = non_empty_string;

                    let mut jumps_to_end = Vec::new();

                    for expr in non_empty_string {
                        self.process_expression(&expr.value, var_ptr.clone())
                            .map_err(add_position(&expr.mark))?;

                        jumps_to_end.push(self.jump_if(Condition::NonEmptyString(var_ptr.clone())));
                    }

                    let noop_index = self.statements.len();
                    self.statements.push(Statement::Noop);

                    for update_jump_to in jumps_to_end {
                        update_jump_to(self, noop_index);
                    }
                }
                VarProcessing::ParseDateToMs(parse_date_to_ms) => {
                    let ParseDateToMsProcessing {
                        parse_date_to_ms: expr,
                    } = parse_date_to_ms;

                    self.process_expression(&expr.value, var_ptr.clone())
                        .map_err(add_position(&expr.mark))?;

                    self.statements
                        .push(Statement::ParseDateToMs(ParseDateToMsStatement {
                            ptr: var_ptr,
                            mark: expr.mark.clone(),
                        }));
                }
                VarProcessing::ParseUint(parse_uint) => {
                    let ParseUintProcessing { parse_uint: expr } = parse_uint;

                    self.process_expression(&expr.value, var_ptr.clone())
                        .map_err(add_position(&expr.mark))?;

                    self.statements.push(Statement::ParseUint { ptr: var_ptr });
                }
                VarProcessing::RegexpReplace(regexp_replace) => {
                    let RegexpReplaceProcessing {
                        regexp_replace: RegexpReplaceProcessingBody { var, from, to },
                    } = regexp_replace;

                    let regexp = Regex::new(&from.value).map_err(|_| {
                        InterpreterError::custom_with_mark(
                            "invalid regexp".to_string(),
                            from.mark.clone(),
                        )
                    })?;

                    let source = self
                        .named_var(&var.value)
                        .map_err(add_position(&var.mark))?;

                    self.statements.push(Statement::Copy {
                        source,
                        destination: var_ptr.clone(),
                    });
                    self.statements.push(Statement::RegexpReplace {
                        ptr: var_ptr,
                        regexp,
                        to: to.clone(),
                    });
                }
                VarProcessing::Unknown(node) => {
                    if let Some(mapping) = node.as_mapping() {
                        if let Some((name_node, _)) = mapping.items.first() {
                            if let Some(name) = name_node.as_str() {
                                return Err(InterpreterError::Custom(ExpectError {
                                    message: format!("unknown var processing: \"{}\"", name),
                                    position: Some(name_node.into()),
                                }));
                            }
                        }
                    }

                    return Err(InterpreterError::Custom(ExpectError {
                        message: "unknown var processing".to_string(),
                        position: Some(node.into()),
                    }));
                }
            }
        }

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
