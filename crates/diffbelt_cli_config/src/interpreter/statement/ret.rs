use crate::code::ReturnValue;
use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{add_position, InterpreterError};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::value::Value;
use crate::interpreter::var::VarDef;
use std::cell::RefCell;
use std::collections::HashMap;
use diffbelt_util::Wrap;

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub source: VarPointer,
}

impl<'a> FunctionInitState<'a> {
    pub fn process_return(&mut self, ret: &ReturnValue) -> Result<(), InterpreterError> {
        let mut cleanups = Cleanups::new();

        let result_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);

        match ret {
            ReturnValue::Var(expr) => {
                self.process_expression(&expr.value, result_ptr.clone())
                    .map_err(add_position(&expr.mark))?;
            }
            ReturnValue::Mapping(mapping) => {
                self.statements.push(Statement::Set {
                    value: Value::Map(Wrap::wrap(HashMap::new())),
                    destination: result_ptr.clone(),
                });

                let tmp_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);

                for (key, expr) in mapping {
                    self.process_expression(&expr.value, tmp_ptr.clone())
                        .map_err(add_position(&expr.mark))?;
                    self.statements.push(Statement::InsertToMap {
                        map_mark: None,
                        map: result_ptr.clone(),
                        key: VarPointer::LiteralStr(key.clone()),
                        value: tmp_ptr.clone(),
                    });
                }
            }
            ReturnValue::Unknown(node) => {
                return Err(InterpreterError::custom_with_mark(
                    "Unknown return type".to_string(),
                    node.into(),
                ));
            }
        }

        self.statements.push(Statement::Return(result_ptr));

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
