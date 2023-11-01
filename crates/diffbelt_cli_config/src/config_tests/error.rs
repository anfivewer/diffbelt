use std::borrow::Cow;
use std::str::Utf8Error;

use thiserror::Error;

use diffbelt_protos::InvalidFlatbuffer;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util_no_std::impl_from_either;
use diffbelt_util_no_std::slice::SliceOffsetError;

use crate::config_tests::value::{ScalarParseError, YamlValueConstructionError};
use crate::formats::human_readable::HumanReadableError;
use crate::wasm::WasmError;

#[derive(Debug)]
pub enum AssertError {
    ValueMissmatch {
        message: Cow<'static, str>,
        expected: Option<String>,
        actual: Option<String>,
    },
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
