mod concat;
mod copy;
mod jump_if;
mod parse_date;
mod regexp;
mod regexp_replace;
mod util;

use crate::interpreter::error::InterpreterError;

use crate::interpreter::function::Function;
use crate::interpreter::statement::Statement;
use crate::interpreter::value::{PrimitiveValue, Value, ValueHolder};
use crate::interpreter::var::{Var, VarDef};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

struct FunctionExecution<'a> {
    pub vars: Vec<Var>,
    pub statement_index: usize,
    pub statements: &'a [Statement],
    pub result: Option<Value>,
}

impl Function {
    pub fn call(&self, mut input_vars: HashMap<Rc<str>, Var>) -> Result<Value, InterpreterError> {
        let Function {
            input_vars_def,
            vars: initial_vars,
            statements,
        } = self;

        let mut rest_variables: HashSet<Rc<str>> =
            input_vars.keys().map(|key| key.clone()).collect();

        let mut vars = initial_vars.clone();

        for (index, (name, _)) in input_vars_def.iter().enumerate() {
            let value = input_vars
                .remove(name)
                .ok_or_else(|| InterpreterError::MissingVariableInFunctionCall(name.clone()))?;

            vars[index] = value;

            rest_variables.remove(name);
        }

        if let Some(name) = rest_variables.iter().next() {
            return Err(InterpreterError::ExtraVariableInFunctionCall(name.clone()));
        }

        let mut state = FunctionExecution {
            vars,
            statement_index: 0,
            statements: &statements,
            result: None,
        };

        loop {
            // let statement = state.current_statement()?;
            //
            state.execute_statement()?;
        }

        todo!()
    }
}

impl<'a> FunctionExecution<'a> {
    pub fn execute_statement(&mut self) -> Result<(), InterpreterError> {
        let statement = self.statements.get(self.statement_index).ok_or_else(|| {
            InterpreterError::custom_without_mark(format!(
                "no statement by index {}",
                self.statement_index
            ))
        })?;

        match statement {
            Statement::Noop => {
                self.statement_index += 1;
                Ok(())
            }
            Statement::Todo(msg) => {
                return Err(InterpreterError::custom_without_mark(format!(
                    "not implemented yet: {}",
                    msg
                )));
            }
            Statement::Copy {
                source,
                destination,
            } => self.execute_copy(source, destination),
            Statement::Set { value, destination } => {
                self.set_var(
                    destination,
                    Var {
                        def: VarDef::unknown(),
                        value: Some(ValueHolder {
                            value: value.clone(),
                        }),
                    },
                )?;

                self.statement_index += 1;
                Ok(())
            }
            Statement::JumpIf(jump_if) => self.execute_jump_if(jump_if),
            Statement::Return(_) => {
                todo!()
            }
            Statement::InsertToMap {
                map_mark,
                map,
                key,
                value,
            } => {
                let map = self.read_var_as_map(map, map_mark.as_ref())?;
                let key = self.read_var_as_rc_str(key, None)?;
                let value = self.read_var_value(value)?;

                {
                    let mut map = map.borrow_mut();
                    map.insert(PrimitiveValue::String(key), value);
                }

                self.statement_index += 1;
                Ok(())
            }
            Statement::PushToList { .. } => {
                todo!()
            }
            Statement::DateFromUnixMs { .. } => {
                todo!()
            }
            Statement::ParseDateToMs(statement) => self.execute_parse_date_to_ms(statement),
            Statement::ParseUint { ptr } => {
                let value = self.read_var_as_str(ptr, None)?;
                let value = str::parse::<u64>(value).map_err(|_| {
                    InterpreterError::custom_without_mark(format!(
                        "parse_uint: not a number \"{}\"",
                        value
                    ))
                })?;

                self.set_var(ptr, Var::new_u64(value))?;
                self.statement_index += 1;
                Ok(())
            }
            Statement::RegexpReplace(statement) => self.execute_regexp_replace(statement),
            Statement::Regexp(regexp) => self.execute_regexp(regexp),
            Statement::Concat(concat) => self.execute_concat(concat),
            Statement::Jump { statement_index } => {
                self.statement_index = *statement_index;
                Ok(())
            }
        }
    }
}
