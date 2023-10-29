use crate::error_code::ErrorCode;
use crate::BytesVecFull;

pub trait HumanReadable {
    extern "C" fn human_readable_key_to_bytes(
        key: BytesVecFull,
        bytes: *mut BytesVecFull,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_key(
        bytes: BytesVecFull,
        key: *mut BytesVecFull,
    ) -> ErrorCode;

    extern "C" fn human_readable_value_to_bytes(
        value: BytesVecFull,
        bytes: *mut BytesVecFull,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_value(
        bytes: BytesVecFull,
        value: *mut BytesVecFull,
    ) -> ErrorCode;
}
