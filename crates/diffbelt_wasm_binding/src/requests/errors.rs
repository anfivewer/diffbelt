use crate::error_code::ErrorCode;
use alloc::string::String;
use core::fmt::{Debug, Formatter};
use diffbelt_protos::align_util::{AlignedBytesError, OwnedAlignedBytes};
use diffbelt_protos::error::FlatbufferError;
use diffbelt_protos::FLATBUFFERS_ALIGNMENT;
use thiserror_no_std::Error;

pub struct RequestErrorWithBuffer {
    pub buffer: Option<OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>>,
    pub error: RequestError,
}

impl Debug for RequestErrorWithBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.error.fmt(f)
    }
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("HostCall({0:?})")]
    HostCall(ErrorCode),
    #[error("{0:?}")]
    AlignedBytes(#[from] AlignedBytesError),
    #[error("{0:?}")]
    Flatbuffer(#[from] FlatbufferError),
    #[error("ResponseError(code = {code}, reason = {reason:?}, details = {details:?})")]
    Response {
        code: i32,
        reason: Option<String>,
        details: Option<String>,
    },
    #[error("Message({0})")]
    MessageStatic(&'static str),
}
