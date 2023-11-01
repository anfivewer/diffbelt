use crate::human_readable::noop;
use crate::util::run_error_coded::run_error_coded;
use alloc::format;
use alloc::string::FromUtf8Error;
use core::str::Utf8Error;
use diffbelt_wasm_binding::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::debug_print_string;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use thiserror_no_std::Error;

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
        key: BytesSlice,
        result_bytes: *mut BytesVecRawParts,
    ) -> ErrorCode {
        () = noop(key, result_bytes);
        ErrorCode::Ok
    }

    #[export_name = "logLinesBytesToKey"]
    extern "C" fn bytes_to_human_readable_key(
        bytes: BytesSlice,
        key: *mut BytesVecRawParts,
    ) -> ErrorCode {
        () = noop(bytes, key);
        ErrorCode::Ok
    }

    #[export_name = "logLinesValueToBytes"]
    extern "C" fn human_readable_value_to_bytes(
        key: BytesSlice,
        bytes: *mut BytesVecRawParts,
    ) -> ErrorCode {
        () = noop(key, bytes);
        ErrorCode::Ok
    }

    #[export_name = "logLinesBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        bytes: BytesSlice,
        key: *mut BytesVecRawParts,
    ) -> ErrorCode {
        () = noop(bytes, key);
        ErrorCode::Ok
    }
}