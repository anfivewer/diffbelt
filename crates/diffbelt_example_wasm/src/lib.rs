#![no_std]

extern crate alloc;

use crate::debug_print::debug_print_string;
use crate::regex::Regex;
use alloc::format;
use alloc::vec::Vec;
use core::ptr::slice_from_raw_parts;
use core::slice;
use core::str::from_utf8;
use diffbelt_protos::protos::transform::map_filter::{
    MapFilterMultiInput, RecordUpdate, RecordUpdateArgs,
};
use diffbelt_protos::{deserialize, Serializer};
use crate::log_lines::parse_log_line;

mod debug_print;
mod global_allocator;
mod panic;
mod regex;
mod log_lines;
mod date;
mod util;

#[export_name = "mapFilter"]
pub extern "C" fn map_filter(input_ptr: *const u8, input_len: i32) -> () {
    let input = unsafe { slice::from_raw_parts(input_ptr, input_len as usize) };

    let result = deserialize::<MapFilterMultiInput>(input).unwrap();

    let items = result.items().expect("no inputs");

    let mut serializer = Serializer::new();
    // let mut record_offsets = Vec::with_capacity(items.len());

    for item in items {
        let is_deleted = item.source_new_value().is_none();

        let source_key = item.source_key().expect("no source key");
        let source_key = from_utf8(source_key.bytes()).expect("source_key is not utf8");

        let parsed = parse_log_line(source_key);

        // if is_deleted {
        //     let key = serializer.create_vector(key.bytes());
        //
        //     // Delete
        //     let record_offset = RecordUpdate::create(
        //         serializer.buffer_builder(),
        //         &RecordUpdateArgs {
        //             key: Some(key),
        //             value: None,
        //         },
        //     );
        //
        //     record_offsets.push(record_offset);
        //
        //     continue;
        // }

        // let source_key = item.source_key().map(|x| x.bytes()).unwrap_or(&[]);
        // let source_key = from_utf8(source_key).unwrap();

        debug_print_string(format!("source_key: {source_key}"));
    }

    let regex = Regex::new(r"^test-(\d+)$");
    let mut mem = Regex::alloc_captures::<2>();
    let Some(captures) = regex.captures("test-42", &mut mem) else {
        return;
    };

    debug_print_string(format!(
        "Captures: {:?}, {:?}",
        captures.get(0),
        captures.get(1)
    ))
}
