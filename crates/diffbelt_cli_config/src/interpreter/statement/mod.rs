pub mod concat;
pub mod regexp;
pub mod vars;
pub mod jump;

use crate::code;
use crate::code::regexp::RegexpInstruction;
use crate::interpreter::error::{ExpectError, InterpreterError};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::concat::ConcatStatement;
use crate::interpreter::statement::jump::JumpIfStatement;
use crate::interpreter::statement::regexp::RegexpStatement;

#[derive(Debug, Clone)]
pub enum Statement {
    Noop,
    Copy {
        source: VarPointer,
        destination: VarPointer,
    },
    JumpIf(JumpIfStatement),

    DateFromUnixMs {
        ptr: VarPointer,
    },

    Regexp(RegexpStatement),
    Concat(ConcatStatement),
}

impl<'a> FunctionInitState<'a> {
    pub fn process_instruction(
        &mut self,
        instruction: &code::Instruction,
    ) -> Result<(), InterpreterError> {
        match instruction {
            code::Instruction::Vars(vars) => self.process_vars_instruction(vars),
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
