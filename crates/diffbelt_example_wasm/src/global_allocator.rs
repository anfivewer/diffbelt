use alloc::vec::Vec;
use dlmalloc::GlobalDlmalloc;

#[global_allocator]
static GLOBAL: GlobalDlmalloc = GlobalDlmalloc;

#[no_mangle]
extern "C" fn alloc(len: i32) -> *mut u8 {
    let vec = Vec::<u8>::with_capacity(len as usize);
    let ptr = vec.leak() as *mut [u8];
    ptr as *mut u8
}

#[no_mangle]
extern "C" fn free(ptr: *mut u8, len: i32) {
    unsafe {
        let _: Vec<u8> = Vec::from_raw_parts(ptr, 0, len as usize);
    }
}
