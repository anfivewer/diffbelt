use crate::util::run_error_coded::run_error_coded;
use alloc::format;
use alloc::string::FromUtf8Error;
use core::str::Utf8Error;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::{debug_print_string, BytesVecFull};
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
    extern "C" fn human_readable_key_to_bytes(
        key: BytesVecFull,
        result_bytes: *mut BytesVecFull,
    ) -> ErrorCode {
        run_error_coded(|| {
            let key = unsafe { key.into_string() }?;

            debug_print_string(format!("to bytes: {key}"));

            Ok::<_, LogLinesError>(ErrorCode::Ok)
        })
    }

    extern "C" fn bytes_to_human_readable_key(
        bytes: BytesVecFull,
        key: *mut BytesVecFull,
    ) -> ErrorCode {
        todo!()
    }

    extern "C" fn human_readable_value_to_bytes(
        key: BytesVecFull,
        bytes: *mut BytesVecFull,
    ) -> ErrorCode {
        todo!()
    }

    extern "C" fn bytes_to_human_readable_value(
        bytes: BytesVecFull,
        key: *mut BytesVecFull,
    ) -> ErrorCode {
        todo!()
    }
}
