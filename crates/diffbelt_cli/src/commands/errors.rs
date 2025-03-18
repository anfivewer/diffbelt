use std::fmt::{Debug, Formatter};
use thiserror::Error;

use diffbelt_cli_config::config_tests::run::RunTestsError;
use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::wasm::error::WasmError;
use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_protos::align_util::AlignedBytesError;
use diffbelt_protos::error::FlatbufferError;
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
    TransformEval(#[from] TransformEvalError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
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

pub struct WasmAggregateMapCallError {
    pub input_buffer: Vec<u8>,
    pub input_head: usize,
    pub input_len: usize,
    pub error: WasmError,
}

impl Debug for WasmAggregateMapCallError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let data = &self.input_buffer[self.input_head..(self.input_head + self.input_len)];
        let encoded = base64::encode(data);

        let () = f.write_fmt(format_args!(
            "Aggregate::map error: {:?}, input data: {encoded}",
            self.error
        ))?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum TransformEvalError {
    #[error("{0}")]
    Unspecified(String),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error("{0:?}")]
    WasmAggregateMapCall(WasmAggregateMapCallError),
    #[error(transparent)]
    Flatbuffer(#[from] NoStdErrorWrap<FlatbufferError>),
    #[error(transparent)]
    InvalidFlatbuffer(#[from] NoStdErrorWrap<InvalidFlatbuffer>),
    #[error(transparent)]
    AlignedBytes(#[from] NoStdErrorWrap<AlignedBytesError>),
}

impl_from_either!(TransformEvalError);

impl From<TransformEvalError> for RunTransformError {
    fn from(value: TransformEvalError) -> Self {
        match value {
            TransformEvalError::Unspecified(msg) => RunTransformError::Message(msg),
            TransformEvalError::Wasm(err) => err.into(),
            TransformEvalError::WasmAggregateMapCall(err) => err.error.into(),
            TransformEvalError::Flatbuffer(err) => err.into(),
            TransformEvalError::InvalidFlatbuffer(err) => err.into(),
            TransformEvalError::AlignedBytes(err) => err.into(),
        }
    }
}

impl From<RunTransformError> for CommandError {
    fn from(value: RunTransformError) -> Self {
        match value {
            RunTransformError::Message(msg) => CommandError::Message(msg),
            RunTransformError::Transform(err) => err.into(),
            RunTransformError::Wasm(err) => err.into(),
            RunTransformError::DiffbeltClient(err) => err.into(),
            RunTransformError::Flatbuffer(err) => TransformEvalError::Flatbuffer(err).into(),
            RunTransformError::InvalidFlatbuffer(err) => {
                TransformEvalError::InvalidFlatbuffer(err).into()
            }
            RunTransformError::AlignedBytes(err) => TransformEvalError::AlignedBytes(err).into(),
        }
    }
}
