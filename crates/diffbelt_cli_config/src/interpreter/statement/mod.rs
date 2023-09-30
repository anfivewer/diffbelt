pub mod concat;
pub mod jump;
pub mod parse_date;
pub mod regexp;
pub mod ret;
pub mod update_list;
pub mod update_map;
pub mod vars;

use crate::code;
use crate::code::regexp::RegexpInstruction;
use crate::code::update_list::UpdateListInstruction;
use crate::code::update_map::UpdateMapInstruction;
use crate::errors::ConfigPositionMark;
use crate::interpreter::error::{ExpectError, InterpreterError};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::concat::ConcatStatement;
use crate::interpreter::statement::jump::JumpIfStatement;
use crate::interpreter::statement::parse_date::ParseDateToMsStatement;
use crate::interpreter::statement::regexp::RegexpStatement;
use crate::interpreter::statement::vars::RegexpReplaceStatement;
use crate::interpreter::value::Value;

#[derive(Debug, Clone)]
pub enum Statement {
    Noop,
    Todo(String),
    Copy {
        source: VarPointer,
        destination: VarPointer,
    },
    Set {
        value: Value,
        destination: VarPointer,
    },
    Jump {
        statement_index: usize,
    },
    JumpIf(JumpIfStatement),
    Return(VarPointer),

    InsertToMap {
        map_mark: Option<ConfigPositionMark>,
        map: VarPointer,
        key: VarPointer,
        value: VarPointer,
    },
    PushToList {
        list_mark: Option<ConfigPositionMark>,
        list: VarPointer,
        value: VarPointer,
    },

    DateFromUnixMs {
        ptr: VarPointer,
    },
    ParseDateToMs(ParseDateToMsStatement),
    ParseUint {
        ptr: VarPointer,
    },
    RegexpReplace(RegexpReplaceStatement),

    Regexp(RegexpStatement),
    Concat(ConcatStatement),
}

impl<'a> FunctionInitState<'a> {
    pub fn process_code(&mut self, code: &code::Code) -> Result<(), InterpreterError> {
        let code::Code { instructions } = code;

        for instruction in instructions {
            () = self.process_instruction(instruction)?;
        }

        Ok(())
    }

    pub fn process_instruction(
        &mut self,
        instruction: &code::Instruction,
    ) -> Result<(), InterpreterError> {
        match instruction {
            code::Instruction::Vars(vars) => self.process_vars_instruction(vars),
            code::Instruction::UpdateMap(instruction) => {
                let UpdateMapInstruction { update_map } = instruction;

                self.process_update_map(update_map)
            }
            code::Instruction::UpdateList(instruction) => {
                let UpdateListInstruction { update_list } = instruction;

                self.process_update_list(update_list)
            }
            code::Instruction::Regexp(regexp) => {
                let RegexpInstruction { regexp } = regexp;

                self.process_regexp(regexp)
            }
            code::Instruction::Return(ret) => self.process_return(&ret.value),
            code::Instruction::Unknown(node) => Err(InterpreterError::Custom(ExpectError {
                message: "unknown instruction".to_string(),
                position: Some(node.into()),
            })),
        }
    }
}
