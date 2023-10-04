use crate::base::input::diffbelt_call::{DiffbeltCallInput, DiffbeltResponseBody};
use crate::base::input::function_eval::FunctionEvalInput;

pub mod diffbelt_call;
pub mod function_eval;

pub struct Input {
    pub id: (u64, u64),
    pub input: InputType,
}

pub enum InputType {
    DiffbeltCall(DiffbeltCallInput<DiffbeltResponseBody>),
    FunctionEval(FunctionEvalInput),
}
