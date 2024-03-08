use crate::annotations::Annotated;
use crate::annotations::InputOutputAnnotated;
use crate::error_code::ErrorCode;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};

pub trait HumanReadable {
    extern "C" fn human_readable_key_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer: *mut BytesVecRawParts,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_key(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    extern "C" fn human_readable_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer: *mut BytesVecRawParts,
    ) -> ErrorCode;
    extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;
}

pub trait AggregateHumanReadable {
    extern "C" fn mapped_key_from_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    extern "C" fn mapped_value_from_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;
}
