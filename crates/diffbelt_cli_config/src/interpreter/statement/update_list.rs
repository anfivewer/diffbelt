use crate::code::update_list::UpdateListInstructionBody;
use crate::code::update_map::UpdateMapInstructionBody;
use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{add_position, InterpreterError};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::VarDef;

impl<'a> FunctionInitState<'a> {
    pub fn process_update_list(
        &mut self,
        instruction: &UpdateListInstructionBody,
    ) -> Result<(), InterpreterError> {
        let UpdateListInstructionBody { var, push } = instruction;

        let mut cleanups = Cleanups::new();

        let list_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);
        () = self
            .process_expression(&var.value, list_ptr.clone())
            .map_err(add_position(&var.mark))?;

        let value_ptr = self.temp_var(VarDef::unknown(), &mut cleanups);
        () = self
            .process_expression(&push.value, value_ptr.clone())
            .map_err(add_position(&push.mark))?;

        self.statements.push(Statement::PushToList {
            list_mark: Some(var.mark.clone()),
            list: list_ptr,
            value: value_ptr,
        });

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
