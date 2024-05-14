use alloc::string::String;

#[link(wasm_import_module = "debug")]
extern "C" {
    fn print(s_ptr: *const u8, s_len: i32) -> ();
}

pub fn debug_print(s: &str) {
    unsafe {
        print(s.as_ptr(), s.len() as i32);
    }
}

pub fn debug_print_string(s: String) {
    let s = s.as_str();
    debug_print(s);
}
