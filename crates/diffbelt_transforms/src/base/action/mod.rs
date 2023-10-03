use crate::base::action::diffbelt_call::DiffbeltCallAction;
use crate::base::action::function_eval::FunctionEvalAction;

pub mod diffbelt_call;
pub mod function_eval;

pub struct Action {
    pub id: (u64, u64),
    pub action: ActionType,
}

pub enum ActionType {
    DiffbeltCall(DiffbeltCallAction),
    FunctionEval(FunctionEvalAction),
}