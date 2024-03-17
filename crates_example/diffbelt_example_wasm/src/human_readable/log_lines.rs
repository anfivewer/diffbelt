use alloc::string::FromUtf8Error;
use core::str::Utf8Error;

use diffbelt_wasm_binding::annotations::{Annotated, InputOutputAnnotated};
use thiserror_no_std::Error;

use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};

struct LogLinesKv;

#[derive(Error, Debug)]
enum LogLinesError {
    Utf8(#[from] Utf8Error),
}

impl From<FromUtf8Error> for LogLinesError {
    fn from(value: FromUtf8Error) -> Self {
        LogLinesError::Utf8(value.utf8_error())
    }
}

impl HumanReadable for LogLinesKv {
    #[export_name = "logLinesKeyToBytes"]
    extern "C" fn human_readable_key_to_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        _buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "logLinesBytesToKey"]
    extern "C" fn bytes_to_human_readable_key(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "logLinesValueToBytes"]
    extern "C" fn human_readable_value_to_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        _buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "logLinesBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }
}
