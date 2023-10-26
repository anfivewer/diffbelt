#[derive(Copy, Clone)]
#[repr(C)]
pub struct MapFilterResult {
    pub result_ptr: *const u8,
    pub result_len: i32,
    pub dealloc_ptr: *mut u8,
    pub dealloc_len: i32,
}

pub trait MapFilter {
    extern "C" fn map_filter(input_ptr: *const u8, input_len: i32) -> *mut MapFilterResult;
}
