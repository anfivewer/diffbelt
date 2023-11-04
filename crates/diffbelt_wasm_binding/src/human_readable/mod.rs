use diffbelt_util_no_std::comments::Annotated;
use crate::bytes::{BytesSlice, BytesVecRawParts};
use crate::error_code::ErrorCode;

pub trait HumanReadable {
    extern "C" fn human_readable_key_to_bytes(
        key: Annotated<BytesSlice, &str>,
        bytes: *mut BytesVecRawParts,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_key(
        bytes: BytesSlice,
        key: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    extern "C" fn human_readable_value_to_bytes(
        value: Annotated<BytesSlice, &str>,
        bytes: *mut BytesVecRawParts,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_value(
        bytes: BytesSlice,
        value: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;
}
