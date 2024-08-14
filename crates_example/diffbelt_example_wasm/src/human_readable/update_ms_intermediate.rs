use crate::global::BUFFER_FOR_REALIGN;
use alloc::string::FromUtf8Error;
use alloc::string::String;
use core::fmt::Write;
use core::str::Utf8Error;
use diffbelt_example_protos::protos::update_ms::{UpdateMsIntermediate, UpdateMsIntermediateArgs};
use diffbelt_protos::align_util::AlignedBytes;
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput;
use diffbelt_protos::{deserialize, Serializer};
use diffbelt_wasm_binding::annotations::serializer::IntoSerializerAnnotated;
use diffbelt_wasm_binding::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use regex::Regex;
use thiserror_no_std::Error;

struct UpdateMsIntermediateKv;

#[derive(Error, Debug)]
enum LogLinesError {
    Utf8(#[from] Utf8Error),
}

impl From<FromUtf8Error> for LogLinesError {
    fn from(value: FromUtf8Error) -> Self {
        LogLinesError::Utf8(value.utf8_error())
    }
}

impl HumanReadable for UpdateMsIntermediateKv {
    #[export_name = "updateMsIntermediateKeyToBytes"]
    extern "C" fn human_readable_key_to_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        _buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "updateMsIntermediateBytesToKey"]
    extern "C" fn bytes_to_human_readable_key(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "updateMsIntermediateValueToBytes"]
    extern "C" fn human_readable_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        let input = unsafe { (&*input_and_output.value).as_str() }.expect("not a string");

        lazy_static::lazy_static! {
            static ref UPDATE_TYPE_RE: Regex = Regex::new(r"^updateType: (.+)$").expect("Cannot build UPDATE_TYPE_RE");
            static ref MS_RE: Regex = Regex::new(r"^ms: (\d+)$").expect("Cannot build UPDATE_TYPE_RE");
        }

        let mut update_type = None;
        let mut ms = None;

        for line in input.lines() {
            if line.is_empty() {
                continue;
            }

            if let Some(captures) = UPDATE_TYPE_RE.captures(line) {
                let m = captures.get(1).expect("no capture");
                update_type = Some(m.as_str());
                continue;
            }

            if let Some(captures) = MS_RE.captures(line) {
                let m = captures.get(1).expect("no capture");
                ms = Some(m.as_str());
                continue;
            }

            panic!("Invalid line: {line}");
        }

        let (Some(update_type), Some(ms)) = (update_type, ms) else {
            panic!("No update_type or ms");
        };

        let ms = ms.parse::<f32>().expect("ms is not a float");

        let buffer_ptr = FlatbufferAnnotated::from(buffer);
        let mut serializer_with_ptr = unsafe { buffer_ptr.into_serializer() };
        let serializer = serializer_with_ptr.serializer_mut();
        let update_type = serializer.create_string(update_type);
        let result = UpdateMsIntermediate::create(
            serializer.buffer_builder(),
            &UpdateMsIntermediateArgs {
                update_type: Some(update_type),
                ms,
            },
        );
        let result = serializer_with_ptr.finish(result);

        unsafe {
            *input_and_output.value = BytesSlice::from(result.serialized_data().value);
            () = result.save();
        }

        ErrorCode::Ok
    }

    #[export_name = "updateMsIntermediateBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        let input = {
            let bytes = unsafe { (&*input_and_output.value).as_slice() };
            let bytes =
                AlignedBytes::ensure_alignment_or_copy(bytes, unsafe { &mut BUFFER_FOR_REALIGN })
                    .expect("align error");
            deserialize::<UpdateMsIntermediate>(bytes).expect("deserialization")
        };

        let output = unsafe { (&*buffer.value).into_empty_vec() };
        let mut output = String::from_utf8(output).expect("vector was empty");

        let update_type = input.update_type().expect("no update_type");
        let ms = input.ms();

        () = output
            .write_fmt(format_args!("updateType: {update_type}\nms: {ms}\n"))
            .expect("fmt");

        ErrorCode::Ok
    }
}
