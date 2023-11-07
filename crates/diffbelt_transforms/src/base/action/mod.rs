use crate::base::action::diffbelt_call::DiffbeltCallAction;
use crate::base::action::function_eval::FunctionEvalAction;

pub mod diffbelt_call;
pub mod function_eval;

#[derive(Debug)]
pub struct Action {
    pub id: (u64, u64),
    pub action: ActionType,
}

#[derive(Debug)]
pub enum ActionType {
    DiffbeltCall(DiffbeltCallAction),
    FunctionEval(FunctionEvalAction),
}

#[cfg(test)]
impl ActionType {
    pub fn as_diffbelt_call(&self) -> Option<&DiffbeltCallAction> {
        let ActionType::DiffbeltCall(call) = self else {
            return None;
        };

        Some(call)
    }
}