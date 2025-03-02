use thiserror::Error;
use std::str::Utf8Error;
use dioxus_hooks::{BorrowError, BorrowMutError};
use diffbelt_protos::align_util::AlignedBytesError;
use diffbelt_protos::error::FlatbufferError;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_wasm_binding::error_code::ErrorCode;

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("AlreadyErrored")]
    AlreadyErrored,
    #[error("{0:?}")]
    Io(std::io::Error),
    #[error("{0:?}")]
    Utf8(Utf8Error),
    #[error("MutexPoisoned")]
    MutexPoisoned,
    #[error("NoMemory")]
    NoMemory,
    #[error("NoAllocation")]
    NoAllocation,
    #[error("DiffbeltRequestSend")]
    DiffbeltRequestSend,
    #[error("{0:?}")]
    Regex(regex::Error),
    #[error("{0:?}")]
    Borrow(#[from] BorrowError),
    #[error("{0:?}")]
    BorrowMut(#[from] BorrowMutError),
    #[error("{0:?}")]
    Flatbuffer(FlatbufferError),
    #[error("BadPointer")]
    BadPointer,
    #[error("{0:?}")]
    WasmTime(#[from] wasmtime::Error),
    #[error("Aggregate::apply error code {0:?}")]
    AggregateApplyErrorCode(ErrorCode),
    #[error(transparent)]
    AlignedBytes(#[from] NoStdErrorWrap<AlignedBytesError>),
    /// Used inside wasm exported functions to show that there is no need to compute anymore
    #[error("NonBrokenTokenCheckFail")]
    NonBrokenTokenCheckFail,
    #[error("{0:?}")]
    Unspecified(String),
}

impl From<FlatbufferError> for WasmError {
    fn from(value: FlatbufferError) -> Self {
        Self::Flatbuffer(value)
    }
}