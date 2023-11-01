use diffbelt_cli_config::config_tests::run::RunTestsError;
use diffbelt_cli_config::interpreter::error::InterpreterError;
use diffbelt_cli_config::wasm::WasmError;
use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_protos::InvalidFlatbuffer;
use diffbelt_transforms::base::error::TransformError;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util_no_std::impl_from_either;
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Unknown")]
    Unknown,
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(std::io::Error),
    #[error(transparent)]
    Utf8(Utf8Error),
    #[error(transparent)]
    RunTests(RunTestsError),
    #[error(transparent)]
    Transform(TransformError),
    #[error(transparent)]
    DiffbeltClient(DiffbeltClientError),
    #[error("{0}")]
    Interpreter(String),
    #[error("{0}")]
    ValueFormat(String),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    MapFilterEval(#[from] MapFilterEvalError),
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

#[derive(Error, Debug)]
pub enum MapFilterEvalError {
    #[error("{0}")]
    Unspecified(String),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    InvalidFlatbuffer(#[from] NoStdErrorWrap<InvalidFlatbuffer>),
}

impl_from_either!(MapFilterEvalError);
