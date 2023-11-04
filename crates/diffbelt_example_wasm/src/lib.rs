#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::str::from_utf8;
use core::{ptr, slice};

use diffbelt_protos::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiOutput, MapFilterMultiOutputArgs, RecordUpdate,
    RecordUpdateArgs,
};
use diffbelt_protos::{deserialize, Serializer};
use diffbelt_wasm_binding::bytes::BytesVecWidePtr;

use crate::log_lines::parse_log_line_header;
use diffbelt_wasm_binding::transform::map_filter::{MapFilter, MapFilterResult};

mod date;
mod global_allocator;
mod human_readable;
mod log_lines;
mod util;

struct LogLinesMapFilter;

impl MapFilter for LogLinesMapFilter {
    #[export_name = "mapFilter"]
    extern "C" fn map_filter(input_ptr: *const u8, input_len: i32) -> *mut MapFilterResult {
        let input = unsafe { slice::from_raw_parts(input_ptr, input_len as usize) };

        let result = deserialize::<MapFilterMultiInput>(input).unwrap();

        let items = result.items().expect("no inputs");

        let mut serializer = Serializer::new();
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

        let result = serializer.finish(result).into_owned();
        let data = result.as_bytes();

        let result_ptr = data.as_ptr();
        let result_len = data.len() as i32;

        let vec = result.into_vec();

        let BytesVecWidePtr {
            ptr: dealloc_ptr,
            capacity: dealloc_len,
        } = BytesVecWidePtr::from(vec);

        static mut STATIC_RESULT: MapFilterResult = MapFilterResult {
            result_ptr: ptr::null_mut(),
            result_len: 0,
            dealloc_ptr: ptr::null_mut(),
            dealloc_len: 0,
        };

        unsafe {
            STATIC_RESULT = MapFilterResult {
                result_ptr: result_ptr as *mut u8,
                result_len,
                dealloc_ptr,
                dealloc_len,
            };

            &mut STATIC_RESULT as *mut MapFilterResult
        }
    }
}
