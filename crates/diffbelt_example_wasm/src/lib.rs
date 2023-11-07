#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::str::from_utf8;

use diffbelt_protos::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiOutput, MapFilterMultiOutputArgs, RecordUpdate,
    RecordUpdateArgs,
};
use diffbelt_wasm_binding::annotations::serializer::{
    InputAnnotated, IntoSerializerAnnotated, OutputAnnotated,
};
use diffbelt_wasm_binding::annotations::{FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::transform::map_filter::MapFilter;

use crate::log_lines::parse_log_line_header;

mod date;
mod global_allocator;
mod human_readable;
mod log_lines;
mod util;

struct LogLinesMapFilter;

impl MapFilter for LogLinesMapFilter {
    #[export_name = "mapFilter"]
    extern "C" fn map_filter(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            MapFilterMultiInput,
            MapFilterMultiOutput,
        >,
        buffer_holder: FlatbufferAnnotated<*mut BytesVecRawParts, MapFilterMultiOutput>,
    ) -> ErrorCode {
        let input = unsafe { input_and_output.deserialize() };

        let items = input.items().expect("no inputs");

        let mut serializer_with_buffer_ptr = unsafe { buffer_holder.into_serializer() };
        let serializer = serializer_with_buffer_ptr.serializer_mut();
        let mut records = Vec::with_capacity(items.len());

        for item in items {
            let is_deleted = item.source_new_value().is_none();

            let source_key = item.source_key().expect("no source key");
            let source_key = from_utf8(source_key.bytes()).expect("source_key is not utf8");

            let Some(parsed) = parse_log_line_header(source_key).expect("invalid log line") else {
                continue;
            };

            let key = serializer.create_vector(parsed.log_line_key.as_bytes());

            if is_deleted {
                // Delete
                let record = RecordUpdate::create(
                    serializer.buffer_builder(),
                    &RecordUpdateArgs {
                        key: Some(key),
                        value: None,
                    },
                );

                records.push(record);

                continue;
            }

            let value = parsed.serialize().expect("invalid log line in rest");
            let value = serializer.create_vector(value.as_bytes());

            let record = RecordUpdate::create(
                serializer.buffer_builder(),
                &RecordUpdateArgs {
                    key: Some(key),
                    value: Some(value),
                },
            );

            records.push(record);
        }

        let records = serializer.create_vector(&records);

        let result = MapFilterMultiOutput::create(
            serializer.buffer_builder(),
            &MapFilterMultiOutputArgs {
                target_update_records: Some(records),
            },
        );

        let serialized = serializer_with_buffer_ptr.finish(result);

        unsafe { input_and_output.save(serialized.serialized_data()) };
        unsafe { serialized.save() };

        ErrorCode::Ok
    }
}
