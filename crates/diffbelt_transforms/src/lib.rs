use crate::base::action::Action;

pub mod base;
pub mod map_filter;
#[cfg(test)]
mod tests;
pub mod util;

#[derive(Debug, Eq, PartialEq)]
pub enum TransformRunResult {
    Actions(Vec<Action>),
    Finish,
}

impl TransformRunResult {
    #[cfg(test)]
    pub fn into_actions(self) -> Result<Vec<Action>, Self> {
        match self {
            TransformRunResult::Actions(actions) => Ok(actions),
            this => Err(this),
        }
    }
}
