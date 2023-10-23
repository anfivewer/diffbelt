#![no_std]

extern crate alloc;

use crate::debug_print::debug_print_string;
use crate::regex::Regex;
use alloc::format;

mod debug_print;
mod global_allocator;
mod panic;
mod regex;

#[export_name = "mapFilter"]
pub extern "C" fn map_filter(input_ptr: *const u8, input_len: i32) -> () {
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
