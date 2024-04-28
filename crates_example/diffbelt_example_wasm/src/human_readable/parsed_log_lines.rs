use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use core::fmt::Write;
use core::mem::forget;
use core::str::Utf8Error;

use thiserror_no_std::Error;

use diffbelt_example_protos::protos::log_line::{ParsedLogLine, ParsedLogLineArgs, Prop, PropArgs};
use diffbelt_protos::{deserialize, InvalidFlatbuffer, Serializer};
use diffbelt_wasm_binding::annotations::{Annotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::Regex;

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

            let buffer = unsafe { (&*bytes).into_empty_vec() };
            let mut serializer = Serializer::<ParsedLogLine>::from_vec(buffer);

            let mut mem = Regex::alloc_captures::<3>();

            let captures = LOG_LEVEL_RE.captures(value, &mut mem).expect("parsing");

            let log_level = captures
                .get(1)
                .expect("capture")
                .chars()
                .next()
                .expect("log_level char") as u8;

            let mut offset = captures.get(0).expect("capture").len();

            let captures = TS_STR_RE
                .captures(&value[offset..], &mut mem)
                .expect("parsing");

            offset += captures.get(0).expect("capture").len();

            let ts_str = captures.get(1).expect("capture");

            let captures = TS_MS_RE
                .captures(&value[offset..], &mut mem)
                .expect("parsing");

            offset += captures.get(0).expect("capture").len();

            let ts_ms = captures.get(1).expect("capture");
            let ts_ms = ts_ms.parse::<u64>().expect("ts_ms parse");

            let captures = TS_MICRO_RE
                .captures(&value[offset..], &mut mem)
                .expect("parsing");

            offset += captures.get(0).expect("capture").len();

            let ts_micro = captures.get(1).expect("capture");
            let ts_micro = ts_micro.parse::<u16>().expect("ts_micro parse");

            let captures = LOGGER_KEY_RE
                .captures(&value[offset..], &mut mem)
                .expect("parsing");

            offset += captures.get(0).expect("capture").len();

            let logger_key = captures.get(1).expect("capture");

            let captures = LOG_KEY_RE
                .captures(&value[offset..], &mut mem)
                .expect("parsing");

            offset += captures.get(0).expect("capture").len();

            let log_key = captures.get(1).expect("capture");

            let captures = PROPS_START_RE.captures(&value[offset..], &mut mem);

            let mut props = Vec::new();

            if let Some(captures) = captures {
                offset += captures.get(0).expect("capture").len();

                loop {
                    let captures = PROP_RE.captures(&value[offset..], &mut mem);

                    if let Some(captures) = captures {
                        let key = captures.get(1).expect("capture");
                        let value = captures.get(2).expect("capture");

                        let key = serializer.create_string(key);
                        let value = serializer.create_string(value);

                        props.push(Prop::create(
                            serializer.buffer_builder(),
                            &PropArgs {
                                key: Some(key),
                                value: Some(value),
                            },
                        ));

                        offset += captures.get(0).expect("capture").len();
                    } else {
                        break;
                    }
                }
            }

            let captures = EXTRA_START_RE.captures(&value[offset..], &mut mem);

            let mut extras = Vec::new();

            if let Some(captures) = captures {
                offset += captures.get(0).expect("capture").len();

                loop {
                    let captures = EXTRA_RE.captures(&value[offset..], &mut mem);

                    if let Some(captures) = captures {
                        let value = captures.get(1).expect("capture");

                        extras.push(serializer.create_string(value));

                        offset += captures.get(0).expect("capture").len();
                    } else {
                        break;
                    }
                }
            }

            let is_no_more = &value[offset..].is_empty();

            if !is_no_more {
                panic!("not parsed {}", &value[offset..]);
            }

            let ts_str = serializer.create_string(ts_str);
            let logger_key = serializer.create_string(logger_key);
            let log_key = serializer.create_string(log_key);
            let props = serializer.create_vector(&props);
            let extras = serializer.create_vector(&extras);

            let parsed_log_line = ParsedLogLine::create(
                serializer.buffer_builder(),
                &ParsedLogLineArgs {
                    log_level,
                    timestamp_string: Some(ts_str),
                    timestamp_milliseconds: ts_ms,
                    timestamp_microseconds: ts_micro,
                    logger_key: Some(logger_key),
                    log_key: Some(log_key),
                    props: Some(props),
                    extra: Some(extras),
                },
            );

            let parsed_log_line = serializer.finish(parsed_log_line);

            unsafe {
                *input_and_output.value = BytesSlice::from(parsed_log_line.as_bytes());
            }

            forget(parsed_log_line);

            Ok(ErrorCode::Ok)
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
