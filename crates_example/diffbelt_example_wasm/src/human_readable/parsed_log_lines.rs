use alloc::format;
use alloc::string::{FromUtf8Error, String};
use core::fmt::Write;
use core::str::{from_utf8, Utf8Error};

use thiserror_no_std::Error;

use diffbelt_example_protos::protos::log_line::{ParsedLogLine, ParsedLogLineBuilder};
use diffbelt_protos::{deserialize, InvalidFlatbuffer, Serializer};
use diffbelt_wasm_binding::annotations::{Annotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::{AggregateHumanReadable, HumanReadable};
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::{debug_print_string, Regex};

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
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        _buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "parsedLogLinesBytesToKey"]
    extern "C" fn bytes_to_human_readable_key(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        run_error_coded(|| Ok::<_, LogLinesError>(ErrorCode::Ok))
    }

    #[export_name = "parsedLogLinesValueToBytes"]
    extern "C" fn human_readable_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        bytes: *mut BytesVecRawParts,
    ) -> ErrorCode {
        lazy_static::lazy_static! {
            static ref LOG_LEVEL_RE: Regex = Regex::new(r"^logLevel: ([A-Z]).*\n").expect("Cannot build LOG_LEVEL_RE");
            static ref TS_STR_RE: Regex = Regex::new(r"^tsStr: (.+)\n").expect("Cannot build TS_STR_RE");
            static ref TS_MS_RE: Regex = Regex::new(r"^tsMs: (\d+)\n").expect("Cannot build TS_MS_RE");
            static ref TS_MICRO_RE: Regex = Regex::new(r"^tsMicro: (\d+)\n").expect("Cannot build TS_MICRO_RE");
            static ref LOGGER_KEY_RE: Regex = Regex::new(r"^loggerKey: (.+)\n").expect("Cannot build LOGGER_KEY_RE");
            static ref LOG_KEY_RE: Regex = Regex::new(r"^logKey: (.+)\n").expect("Cannot build LOG_KEY_RE");
            static ref PROPS_START_RE: Regex = Regex::new(r"^props:\n").expect("Cannot build PROPS_START_RE");
            static ref PROP_RE: Regex = Regex::new(r"^  ((?:[^:]|:[^ ])*): (.*)\n").expect("Cannot build PROP_RE");
            static ref EXTRA_START_RE: Regex = Regex::new(r"^extra:\n").expect("Cannot build EXTRA_START_RE");
            static ref EXTRA_RE: Regex = Regex::new(r"^  (.*)\n").expect("Cannot build EXTRA_RE");
        }

        run_error_coded(|| -> Result<ErrorCode, LogLinesError> {
            let value = unsafe { (&*input_and_output.value).as_str() }?;

            let q = from_utf8(value.as_bytes())
                .map_or_else(|err| Some(err), |_| None)
                .map(|x| format!("{:#?}", x));
            let _q = q.as_ref().map(|x| x.as_str()).unwrap_or("No error");

            let q = from_utf8(value.as_bytes())
                .map_or_else(|err| Some(err), |_| None)
                .map(|x| format!("{:#?}", x));
            let _q = q.as_ref().map(|x| x.as_str()).unwrap_or("No error");

            let buffer = unsafe { (&*bytes).into_empty_vec() };
            let mut serializer = Serializer::<ParsedLogLine>::from_vec(buffer);

            let _builder = ParsedLogLineBuilder::new(serializer.buffer_builder());

            let mut mem = Regex::alloc_captures::<2>();

            let log_level_captures = LOG_LEVEL_RE.captures(value, &mut mem).expect("parsing");
            let offset = log_level_captures.get(0).expect("capture").len();

            debug_print_string(format!(
                "logLevel {}, rest: ({}) {}",
                log_level_captures.get(1).expect("capture"),
                offset,
                value,
            ));

            let ts_str_captures = TS_STR_RE
                .captures(&value[offset..], &mut mem)
                .expect("parsing");

            debug_print_string(format!(
                "tsStr {}",
                ts_str_captures.get(1).expect("capture")
            ));

            /*
            logLevel: S (83)
            tsStr: 2023-02-20T21:42:48.822Z.000
            tsMs: 1676929368822
            tsMicro: 0
            loggerKey: worker258688:middlewares
            logKey: handleFull
            props:
              updateType: edited_message
              ms: 27
            extra:
              some extra 2
              another extra 2
                         */

            // Ok::<_, LogLinesError>(ErrorCode::Ok)
            todo!()
        })
    }

    #[export_name = "parsedLogLinesBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        key: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        run_error_coded(|| {
            let vec = unsafe { (&*key.value).into_empty_vec() };
            let mut s = String::from_utf8(vec).expect("empty vec should be valid string");

            let bytes = unsafe { (&*input_and_output.value).as_slice() };
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
                        let key = prop
                            .key()
                            .map(|x| x.replace("\\", "\\\\").replace(": ", ":\\ "));

                        () = s.write_fmt(format_args!(
                            "  {}: {}\n",
                            key.as_ref().map(|x| x.as_str()).unwrap_or("None"),
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

            unsafe {
                *key.value = s.into_bytes().into();
                *input_and_output.value = (&*key.value).into();
            };

            Ok::<_, LogLinesError>(ErrorCode::Ok)
        })
    }
}

impl AggregateHumanReadable for ParsedLogLinesKv {
    #[export_name = "parsedLogLinesMappedKeyFromBytes"]
    extern "C" fn mapped_key_from_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "parsedLogLinesMappedValueFromBytes"]
    extern "C" fn mapped_value_from_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }
}
