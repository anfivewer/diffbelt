use crate::code::condition::{ConditionInstructionBody};
use crate::interpreter::cleanups::Cleanups;
use crate::interpreter::error::{add_position, InterpreterError};
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::jump::Condition;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::VarDef;

impl<'a> FunctionInitState<'a> {
    pub fn process_condition(
        &mut self,
        condition: &ConditionInstructionBody,
    ) -> Result<(), InterpreterError> {
        let ConditionInstructionBody { condition, then } = condition;

        let mut cleanups = Cleanups::new();

        let condition_ptr = self.temp_var(VarDef::anonymous_bool(), &mut cleanups);
        () = self
            .process_expression(&condition.value, condition_ptr.clone())
            .map_err(add_position(&condition.mark))?;

        let jump_if = self.jump_if(Condition::IsFalse(condition_ptr));

        self.process_code(then)?;

        let last_statement_index = self.statements.len();
        self.statements.push(Statement::Noop);

        jump_if(self, last_statement_index);

        self.apply_cleanups(cleanups)?;

        Ok(())
    }
}
