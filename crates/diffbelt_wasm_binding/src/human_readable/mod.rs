use crate::bytes::{BytesSlice, BytesVecRawParts};
use crate::error_code::ErrorCode;

pub trait HumanReadable {
    extern "C" fn human_readable_key_to_bytes(
        key: BytesSlice,
        bytes: *mut BytesVecRawParts,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_key(
        bytes: BytesSlice,
        key: *mut BytesVecRawParts,
    ) -> ErrorCode;

    extern "C" fn human_readable_value_to_bytes(
        value: BytesSlice,
        bytes: *mut BytesVecRawParts,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_value(
        bytes: BytesSlice,
        value: *mut BytesVecRawParts,
    ) -> ErrorCode;
}
