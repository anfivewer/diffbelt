use crate::annotations::Annotated;
use crate::annotations::InputOutputAnnotated;
use crate::error_code::ErrorCode;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};

pub trait HumanReadable {
    unsafe extern "C" fn human_readable_key_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;
    unsafe extern "C" fn bytes_to_human_readable_key(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer_ptr: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    unsafe extern "C" fn human_readable_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;
    unsafe extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer_ptr: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;
}

pub trait AggregateHumanReadable {
    unsafe extern "C" fn bytes_to_target_key(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer_ptr: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    unsafe extern "C" fn bytes_to_mapped_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer_ptr: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    unsafe extern "C" fn mapped_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;

    unsafe extern "C" fn bytes_to_accumulator(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer_ptr: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode;

    unsafe extern "C" fn accumulator_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;
}
