use crate::code::update_map::UpdateMapInstructionBody;
use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{add_position, InterpreterError};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::VarDef;

impl<'a> FunctionInitState<'a> {
    pub fn process_update_map(
        &mut self,
        instruction: &UpdateMapInstructionBody,
    ) -> Result<(), InterpreterError> {
        let UpdateMapInstructionBody { var, key, value } = instruction;

        let mut cleanups = Cleanups::new();

        let map_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);
        () = self
            .process_expression(&var.value, map_ptr.clone())
            .map_err(add_position(&var.mark))?;

        let key_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);
        () = self
            .process_expression(&key.value, key_ptr.clone())
            .map_err(add_position(&key.mark))?;

        let value_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);
        () = self
            .process_expression(&value.value, value_ptr.clone())
            .map_err(add_position(&value.mark))?;

        self.statements.push(Statement::InsertToMap {
            map_mark: Some(var.mark.clone()),
            map: map_ptr,
            key: key_ptr,
            value: value_ptr,
        });

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
