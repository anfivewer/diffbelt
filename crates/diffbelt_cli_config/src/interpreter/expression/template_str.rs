use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;

impl<'a> FunctionInitState<'a> {
    pub fn process_template_str(
        &mut self,
        template: &str,
    ) -> Result<VarPointer, InterpreterError> {
        println!("template str: {}", template);

        todo!()
    }
}