use crate::errors::ConfigPositionMark;

#[derive(Debug)]
pub enum InterpreterError {
    NoSuchVariable(String),
    InvalidTemplate,
    Custom(ExpectError),
}

#[derive(Debug)]
pub struct ExpectError {
    pub message: String,
    pub position: Option<ConfigPositionMark>,
}

pub fn add_position(
    mark: &ConfigPositionMark,
) -> (impl Fn(InterpreterError) -> InterpreterError + '_) {
    |error| match error {
        InterpreterError::NoSuchVariable(name) => InterpreterError::Custom(ExpectError {
            message: format!("No such variable \"{}\"", name),
            position: Some(mark.clone()),
        }),
        InterpreterError::InvalidTemplate => InterpreterError::Custom(ExpectError {
            message: "Invalid template".to_string(),
            position: Some(mark.clone()),
        }),
        err => err,
    }
}
