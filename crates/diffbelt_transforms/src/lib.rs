use crate::base::action::Action;
use crate::base::error::TransformError;

pub mod base;
pub mod map_filter;

pub enum TransformRunResult {
    Actions(Vec<Action>),
    Finish,
    Error(TransformError),
}
