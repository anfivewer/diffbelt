use crate::base::action::Action;

pub mod aggregate;
pub mod base;
pub mod map_filter;
#[cfg(test)]
mod tests;
mod transform;
pub mod util;

pub use transform::{Transform, TransformImpl};

#[derive(Debug)]
pub enum TransformRunResult {
    Actions(Vec<Action>),
    Finish,
}

#[cfg(test)]
impl TransformRunResult {
    pub fn is_finish(&self) -> bool {
        let Self::Finish = self else {
            return false;
        };

        true
    }
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
