use thiserror::Error;

use diffbelt_cli_config::config_tests::run::RunTestsError;
use diffbelt_cli_config::wasm::WasmError;
use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_protos::InvalidFlatbuffer;
use diffbelt_transforms::base::error::TransformError;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util_no_std::impl_from_either;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    RunTests(RunTestsError),
    #[error(transparent)]
    Transform(TransformError),
    #[error(transparent)]
    DiffbeltClient(DiffbeltClientError),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    MapFilterEval(#[from] TransformEvalError),
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

#[derive(Error, Debug)]
pub enum TransformEvalError {
    #[error("{0}")]
    Unspecified(String),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    InvalidFlatbuffer(#[from] NoStdErrorWrap<InvalidFlatbuffer>),
}

impl_from_either!(TransformEvalError);
