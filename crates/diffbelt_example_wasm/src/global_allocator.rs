use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr;
use diffbelt_wasm_binding::{BytesVecFull, BytesVecPtr, VecAllocator};
use dlmalloc::GlobalDlmalloc;

#[global_allocator]
static GLOBAL: GlobalDlmalloc = GlobalDlmalloc;

struct WasmAllocator;

impl VecAllocator for WasmAllocator {
    #[no_mangle]
    extern "C" fn alloc(capacity: i32) -> BytesVecPtr {
        let vec = Vec::<u8>::with_capacity(capacity as usize);
        let ptr = vec.leak() as *mut [u8];
        BytesVecPtr { ptr: ptr as *mut u8 }
    }

    #[no_mangle]
    unsafe extern "C" fn dealloc(ptr: BytesVecPtr, capacity: i32) {
        unsafe {
            let _: Vec<u8> = from_raw_parts(ptr.ptr, 0, capacity);
        }
    }

    #[no_mangle]
    extern "C" fn alloc_bytes_vec_full_struct() -> *mut BytesVecFull {
        let b = Box::new(BytesVecFull {
            ptr: ptr::null_mut(),
            len: 0,
            capacity: 0,
        });
        Box::leak(b)
    }

    #[no_mangle]
    unsafe extern "C" fn dealloc_bytes_vec_full_struct(ptr: *mut BytesVecFull) {
        let b = Box::from_raw(ptr);
        drop(b);
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
