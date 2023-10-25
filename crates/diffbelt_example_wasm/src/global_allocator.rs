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
extern "C" fn dealloc(ptr: *mut u8, len: i32) {
    unsafe {
        let _: Vec<u8> = from_raw_parts(ptr, 0, len);
    }
}

pub fn leak_vec(vec: Vec<u8>) -> (*mut u8, i32) {
    let capacity = vec.capacity();
    let ptr = vec.leak() as *mut [u8];
    (ptr as *mut u8, capacity as i32)
}

pub unsafe fn from_raw_parts(ptr: *mut u8, len: i32, capacity: i32) -> Vec<u8> {
    Vec::from_raw_parts(ptr, len as usize, capacity as usize)
}
