pub mod regexp;

use crate::code;
use crate::code::regexp::RegexpInstruction;
use crate::interpreter::error::{ExpectError, InterpreterError};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::regexp::RegexpStatement;

#[derive(Debug, Clone)]
pub enum Statement {
    Copy {
        source: VarPointer,
        destination: VarPointer,
    },
    FreeTempVar(usize),

    Regexp(RegexpStatement),
}

impl<'a> FunctionInitState<'a> {
    pub fn process_instruction(
        &mut self,
        instruction: &code::Instruction,
    ) -> Result<(), InterpreterError> {
        match instruction {
            code::Instruction::Vars(_) => {
                todo!()
            }
            code::Instruction::UpdateMap(_) => {
                todo!()
            }
            code::Instruction::Regexp(regexp) => {
                let RegexpInstruction { regexp } = regexp;

                self.process_regexp(regexp)
            }
            code::Instruction::Return(_) => {
                todo!()
            }
            code::Instruction::Unknown(node) => Err(InterpreterError::Custom(ExpectError {
                message: "unknown instruction".to_string(),
                position: Some(node.into()),
            })),
        }
    }
}
