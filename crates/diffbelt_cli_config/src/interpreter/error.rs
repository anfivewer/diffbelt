use crate::errors::ConfigPositionMark;

#[derive(Debug)]
pub enum InterpreterError {
    NoSuchVariable(String),
    Custom(ExpectError),
}

#[derive(Debug)]
pub struct ExpectError {
    pub message: String,
    pub position: Option<ConfigPositionMark>,
}

pub fn add_position_to_no_such_variable(
    mark: &ConfigPositionMark,
) -> (impl Fn(InterpreterError) -> InterpreterError + '_) {
    |error| match error {
        InterpreterError::NoSuchVariable(name) => InterpreterError::Custom(ExpectError {
            message: format!("No such variable \"{}\"", name),
            position: Some(mark.clone()),
        }),
        err => err,
    }
}
