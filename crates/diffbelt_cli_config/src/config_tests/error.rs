use thiserror::Error;
use diffbelt_protos::InvalidFlatbuffer;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util::slice::SliceOffsetError;
use std::str::Utf8Error;
use either::Either;
use crate::config_tests::value::{ScalarParseError, YamlValueConstructionError};
use crate::formats::human_readable::HumanReadableError;
use crate::impl_from_either;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::value::Value;
use crate::wasm::WasmError;

#[derive(Debug)]
pub enum AssertError {
    ValueMissmatch { expected: Value, actual: Value },
}

#[derive(Error, Debug)]
pub enum TestError {
    #[error("InvalidName")]
    InvalidName,
    #[error("SourceHasNoHumanReadableFunctions")]
    SourceHasNoHumanReadableFunctions,
    #[error("TargetHasNoHumanReadableFunctions")]
    TargetHasNoHumanReadableFunctions,
    #[error("{0}")]
    Unspecified(String),
    #[error("{0}")]
    Panic(String),
    #[error("{0:?}")]
    YamlValueConstruction(YamlValueConstructionError),
    #[error("{0:?}")]
    Interpreter(InterpreterError),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    SliceOffset(#[from] NoStdErrorWrap<SliceOffsetError>),
    #[error("{0:?}")]
    InvalidFlatbuffer(InvalidFlatbuffer),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
    #[error(transparent)]
    HumanReadable(#[from] HumanReadableError),
    #[error(transparent)]
    YamlTestVars(#[from] YamlTestVarsError),
}

impl_from_either!(TestError);

#[derive(Error, Debug)]
pub enum YamlTestVarsError {
    #[error(transparent)]
    ScalarParse(#[from] ScalarParseError),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error("{0}")]
    Unspecified(String),
}

impl_from_either!(YamlTestVarsError);