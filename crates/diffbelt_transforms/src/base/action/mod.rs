use crate::base::action::diffbelt_call::DiffbeltCallAction;
use crate::base::action::function_eval::FunctionEvalAction;

pub mod diffbelt_call;
pub mod function_eval;

#[derive(Debug, Eq, PartialEq)]
pub struct Action {
    pub id: (u64, u64),
    pub action: ActionType,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ActionType {
    DiffbeltCall(DiffbeltCallAction),
    FunctionEval(FunctionEvalAction),
}