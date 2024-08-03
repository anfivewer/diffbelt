use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use core::format_args;
use core::str::from_utf8;

use chrono::{Datelike, NaiveDateTime};
use regex::Regex;

use diffbelt_example_protos::protos::log_line::ParsedLogLine;
use diffbelt_example_protos::protos::update_ms::{UpdateMsIntermediate, UpdateMsIntermediateArgs};
use diffbelt_protos::align_util::AlignedBytes;
use diffbelt_protos::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiOutput, MapFilterMultiOutputArgs, RecordUpdate,
    RecordUpdateArgs,
};
use diffbelt_protos::{deserialize, Serialized, Serializer};
use diffbelt_util_no_std::cast::try_u64_to_i64;
use diffbelt_wasm_binding::annotations::serializer::{IntoSerializerAnnotated, OutputAnnotated};
use diffbelt_wasm_binding::annotations::{FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::transform::map_filter::MapFilter;

use crate::global::{BUFFER_FOR_REALIGN, BUFFER_FOR_REALIGN_2};

struct UpdateMsDayIntermediate;

impl<'t> MapFilter for UpdateMsDayIntermediate {
    #[export_name = "updateMs1dIntermediateMapFilter"]
    extern "C" fn map_filter(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            MapFilterMultiInput,
            MapFilterMultiOutput,
        >,
        buffer_holder: FlatbufferAnnotated<*mut BytesVecRawParts, MapFilterMultiOutput>,
    ) -> ErrorCode {
        let input = {
            let bytes = unsafe { (&*input_and_output.value).as_slice() };
            let bytes =
                AlignedBytes::ensure_alignment_or_copy(bytes, unsafe { &mut BUFFER_FOR_REALIGN })
                    .expect("align error");
            deserialize::<MapFilterMultiInput>(bytes).expect("deserialization")
        };

        let items = input.items().unwrap_or_else(|| Default::default());

        let mut serializer_with_buffer_ptr = unsafe { buffer_holder.into_serializer() };
        let serializer = serializer_with_buffer_ptr.serializer_mut();
        let mut records = Vec::with_capacity(items.len());

        let mut old_key = String::new();
        let mut new_key = String::new();
        let mut temp_buffer = Some(Vec::new());

        for item in items {
            let source_key = from_utf8(item.source_key().expect("no source key").bytes())
                .expect("source key is not a string");
            let source_old_value = item.source_old_value().map(|x| x.bytes());
            let intermediate_old_key = source_old_value.and_then(|x| {
                let (has_key, _) = value_to_key(x, source_key, &mut old_key, None);
                if has_key {
                    Some(old_key.as_str())
                } else {
                    None
                }
            });

            let source_new_value = item.source_new_value().map(|x| x.bytes());
            let intermediate_new = source_new_value.and_then(|x| {
                let (has_key, intermediate) =
                    value_to_key(x, source_key, &mut new_key, temp_buffer.take());
                if has_key {
                    Some((new_key.as_str(), intermediate.expect("no intermediate")))
                } else {
                    None
                }
            });

            if let Some(old_key) = intermediate_old_key {
                if let Some((new_key, _)) = intermediate_new.as_ref() {
                    let new_key = *new_key;
                    if old_key != new_key {
                        let key = serializer.create_vector(old_key.as_bytes());
                        let record = RecordUpdate::create(
                            serializer.buffer_builder(),
                            &RecordUpdateArgs {
                                key: Some(key),
                                value: None,
                            },
                        );
                        records.push(record);
                    }
                }
            }

            if let Some((key, intermediate)) = intermediate_new {
                let key = serializer.create_vector(key.as_bytes());
                let value = serializer.create_vector(intermediate.as_bytes());
                let record = RecordUpdate::create(
                    serializer.buffer_builder(),
                    &RecordUpdateArgs {
                        key: Some(key),
                        value: Some(value),
                    },
                );
                records.push(record);

                let mut buffer = intermediate.into_buffer();
                buffer.clear();
                temp_buffer = Some(buffer);
            }
        }

        let result = MapFilterMultiOutput::create(
            serializer.buffer_builder(),
            &MapFilterMultiOutputArgs {
                target_update_records: None,
            },
        );

        let serialized = serializer_with_buffer_ptr.finish(result);

        unsafe { input_and_output.save(serialized.serialized_data()) };
        unsafe { serialized.save() };

        ErrorCode::Ok
    }
}

type HasKey = bool;
fn value_to_key(
    bytes: &[u8],
    source_key: &str,
    key_output: &mut String,
    intermediate_buffer: Option<Vec<u8>>,
) -> (
    HasKey,
    Option<Serialized<'static, UpdateMsIntermediate<'static>>>,
) {
    let bytes = AlignedBytes::ensure_alignment_or_copy(bytes, unsafe { &mut BUFFER_FOR_REALIGN_2 })
        .expect("align error");
    let parsed_log_line = deserialize::<ParsedLogLine>(bytes).expect("deserialization");

    lazy_static::lazy_static! {
        static ref MIDDLEWARE_RE: Regex = Regex::new(r"^worker\d*:middlewares$").expect("Cannot build MIDDLEWARE_RE");
    }

    let logger_key = parsed_log_line.logger_key().expect("no logger_key");

    if parsed_log_line.log_level() != ('S' as u8)
        || parsed_log_line.log_key().unwrap_or("") != "handleFull"
        || MIDDLEWARE_RE.is_match(logger_key)
    {
        return (false, None);
    }

    let mut update_type = None;
    let mut ms = None;

    for prop in parsed_log_line.props().unwrap_or_default() {
        let key = prop.key().unwrap_or_default();
        let mut found = true;

        match key {
            "ms" => {
                ms = prop.value();
            }
            "updateType" => {
                update_type = prop.value();
            }
            _ => {
                found = false;
            }
        }

        if found && ms.is_some() && update_type.is_some() {
            break;
        }
    }

    let Some(ms) = ms else {
        return (false, None);
    };

    let ms = ms.parse::<f32>().expect("invalid ms value");

    let time = try_u64_to_i64(parsed_log_line.timestamp_milliseconds()).expect("too big is");
    let time = NaiveDateTime::from_timestamp_millis(time).expect("invalid ts");

    key_output.clear();

    () = key_output
        .write_fmt(format_args!(
            "{:0>4}-{:0>4}-{:0>4} {:0>11.1} ",
            time.year(),
            time.month(),
            time.day(),
            ms
        ))
        .expect("cannot fmt");

    assert_eq!(
        key_output.len(),
        "2024-08-02 000000000.0 ".len(),
        "Invalid format"
    );

    () = key_output.write_str(source_key).expect("cannot write_str");

    let intermediate = if let Some(buffer) = intermediate_buffer {
        let mut serializer = Serializer::from_vec(buffer);
        let update_type = update_type.unwrap_or("???");
        let update_type = serializer.create_string(update_type);
        let intermediate = UpdateMsIntermediate::create(
            serializer.buffer_builder(),
            &UpdateMsIntermediateArgs {
                update_type: Some(update_type),
                ms,
            },
        );

        Some(serializer.finish(intermediate))
    } else {
        None
    };

    (true, intermediate)
}
