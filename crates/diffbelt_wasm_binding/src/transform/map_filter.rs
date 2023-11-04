use crate::bytes::{BytesSlice, BytesVecRawParts};
use crate::error_code::ErrorCode;

pub trait MapFilter {
    extern "C" fn map_filter(input_and_output: *mut BytesSlice, buffer: *mut BytesVecRawParts) -> ErrorCode;
}
