use diffbelt_cli_config::config_tests::run::RunTestsError;
use diffbelt_cli_config::formats::ValueFormatError;
use diffbelt_cli_config::interpreter::error::InterpreterError;
use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_transforms::base::error::TransformError;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum CommandError {
    Unknown,
    Message(String),
    Io(std::io::Error),
    Utf8(Utf8Error),
    RunTests(RunTestsError),
    Transform(TransformError),
    DiffbeltClient(DiffbeltClientError),
    Interpreter(String),
    ValueFormat(String),
}

impl From<TransformError> for CommandError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<DiffbeltClientError> for CommandError {
    fn from(value: DiffbeltClientError) -> Self {
        Self::DiffbeltClient(value)
    }
}

impl From<InterpreterError> for CommandError {
    fn from(value: InterpreterError) -> Self {
        Self::Interpreter(format!("{value:?}"))
    }
}

impl From<ValueFormatError> for CommandError {
    fn from(value: ValueFormatError) -> Self {
        Self::ValueFormat(format!("{value:?}"))
    }
}
