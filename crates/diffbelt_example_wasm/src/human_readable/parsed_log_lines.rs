use alloc::string::{FromUtf8Error, String};
use core::fmt::Write;
use core::str::Utf8Error;

use thiserror_no_std::Error;

use diffbelt_example_protos::protos::log_line::ParsedLogLine;
use diffbelt_protos::{deserialize, InvalidFlatbuffer};
use diffbelt_util_no_std::comments::Annotated;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};

use crate::util::run_error_coded::run_error_coded;

struct ParsedLogLinesKv;

#[derive(Error, Debug)]
enum LogLinesError {
    Utf8(#[from] Utf8Error),
    Flatbuffer(#[from] InvalidFlatbuffer),
    Fmt(#[from] core::fmt::Error),
}

impl From<FromUtf8Error> for LogLinesError {
    fn from(value: FromUtf8Error) -> Self {
        LogLinesError::Utf8(value.utf8_error())
    }
}

impl HumanReadable for ParsedLogLinesKv {
    #[export_name = "parsedLogLinesKeyToBytes"]
    extern "C" fn human_readable_key_to_bytes(
        _key: Annotated<BytesSlice, &str>,
        _result_bytes: *mut BytesVecRawParts,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "parsedLogLinesBytesToKey"]
    extern "C" fn bytes_to_human_readable_key(
        bytes: BytesSlice,
        key: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        run_error_coded(|| {
            let mut vec = unsafe { (&*key.value).into_empty_vec() };
            vec.extend_from_slice(unsafe { bytes.as_slice() });
            unsafe { *key.value = vec.into() };

            Ok::<_, LogLinesError>(ErrorCode::Ok)
        })
    }

    #[export_name = "parsedLogLinesValueToBytes"]
    extern "C" fn human_readable_value_to_bytes(
        _key: Annotated<BytesSlice, &str>,
        _bytes: *mut BytesVecRawParts,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "parsedLogLinesBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        bytes: BytesSlice,
        key: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        run_error_coded(|| {
            let vec = unsafe { (&*key.value).into_empty_vec() };
            let mut s = String::from_utf8(vec).expect("empty vec should be valid string");

            let bytes = unsafe { bytes.as_slice() };
            let log_line = deserialize::<ParsedLogLine>(bytes)?;

            let log_level = log_line.log_level();

            () = s.write_fmt(format_args!(
                "logLevel: {} ({})\n",
                log_level as char, log_level
            ))?;
            () = s.write_fmt(format_args!(
                "tsStr: {}\n",
                log_line.timestamp_string().unwrap_or("None")
            ))?;
            () = s.write_fmt(format_args!(
                "tsMs: {}\n",
                log_line.timestamp_milliseconds()
            ))?;
            () = s.write_fmt(format_args!(
                "tsMicro: {}\n",
                log_line.timestamp_microseconds()
            ))?;
            () = s.write_fmt(format_args!(
                "loggerKey: {}\n",
                log_line.logger_key().unwrap_or("None")
            ))?;
            () = s.write_fmt(format_args!(
                "logKey: {}\n",
                log_line.log_key().unwrap_or("None")
            ))?;

            if let Some(props) = log_line.props() {
                if !props.is_empty() {
                    s.push_str("props:\n");
                    for prop in props {
                        () = s.write_fmt(format_args!(
                            "  {}: {}\n",
                            prop.key().unwrap_or("None"),
                            prop.value().unwrap_or("None"),
                        ))?;
                    }
                }
            }

            if let Some(extras) = log_line.extra() {
                if !extras.is_empty() {
                    s.push_str("extra:\n");
                    for extra in extras {
                        () = s.write_fmt(format_args!("  {extra}\n"))?;
                    }
                }
            }

            unsafe { *key.value = s.into_bytes().into() };
            Ok::<_, LogLinesError>(ErrorCode::Ok)
        })
    }
}
