use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};

mod log_lines;
mod parsed_log_lines;

pub fn noop(input: BytesSlice, output: *mut BytesVecRawParts) {
    let mut vec = unsafe { (&*output).into_empty_vec() };
    vec.extend_from_slice(unsafe { input.as_slice() });
    unsafe { *output = vec.into() };
}
