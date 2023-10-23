#![no_std]

extern crate alloc;

use crate::debug_print::debug_print_string;
use crate::regex::Regex;
use alloc::format;
use core::ptr::slice_from_raw_parts;
use core::slice;
use core::str::from_utf8;
use diffbelt_protos::deserialize;
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput;

mod debug_print;
mod global_allocator;
mod panic;
mod regex;

#[export_name = "mapFilter"]
pub extern "C" fn map_filter(input_ptr: *const u8, input_len: i32) -> () {
    let input = unsafe { slice::from_raw_parts(input_ptr, input_len as usize) };

    let result = deserialize::<MapFilterMultiInput>(input).unwrap();

    debug_print_string(format!("input {result:?}"));

    let Some(items) = result.items() else {
        return;
    };

    for item in items {
        let source_key = item.source_key().map(|x| x.bytes()).unwrap_or(&[]);
        let source_key = from_utf8(source_key).unwrap();

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
