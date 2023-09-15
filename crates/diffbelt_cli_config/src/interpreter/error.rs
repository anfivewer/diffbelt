use crate::errors::ConfigPositionMark;
use std::rc::Rc;

#[derive(Debug)]
pub enum InterpreterError {
    NoSuchVariable(String),
    InvalidTemplate,
    InvalidExpression(String),
    MissingVariableInFunctionCall(Rc<str>),
    ExtraVariableInFunctionCall(Rc<str>),
    Custom(ExpectError),
}

#[derive(Debug)]
pub struct ExpectError {
    pub message: String,
    pub position: Option<ConfigPositionMark>,
}

impl InterpreterError {
    pub fn custom(message: String, mark: Option<ConfigPositionMark>) -> Self {
        InterpreterError::Custom(ExpectError {
            message,
            position: mark,
        })
    }

    pub fn custom_with_mark(message: String, mark: ConfigPositionMark) -> Self {
        InterpreterError::Custom(ExpectError {
            message,
            position: Some(mark),
        })
    }

    pub fn custom_without_mark(message: String) -> Self {
        InterpreterError::Custom(ExpectError {
            message,
            position: None,
        })
    }
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
        InterpreterError::InvalidExpression(s) => InterpreterError::Custom(ExpectError {
            message: format!("Invalid expression: \"{}\"", s),
            position: Some(mark.clone()),
        }),
        err => err,
    }
}
