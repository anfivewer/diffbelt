use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr;

use crate::ptr::bytes::{BytesSlice, BytesVecPtr, BytesVecRawParts, BytesVecWidePtr};
use crate::ptr::{ConstPtr, MutPtr};

#[no_mangle]
extern "C" fn alloc(capacity: i32) -> BytesVecPtr {
    let vec = Vec::<u8>::with_capacity(capacity as usize);
    let ptr = vec.leak() as *mut [u8];
    BytesVecPtr {
        ptr: ptr as *mut u8,
    }
}

#[no_mangle]
unsafe extern "C" fn dealloc(ptr: BytesVecWidePtr) {
    unsafe {
        let _: Vec<u8> = ptr.into_empty_vec();
    }
}

#[no_mangle]
extern "C" fn alloc_bytes_slice() -> *mut BytesSlice {
    let b = Box::new(BytesSlice {
        ptr: ConstPtr::from(ptr::null()),
        len: 0,
    });
    Box::leak(b)
}

#[no_mangle]
unsafe extern "C" fn dealloc_bytes_slice(ptr: *mut BytesSlice) {
    let b = Box::from_raw(ptr);
    drop(b);
}

#[no_mangle]
extern "C" fn alloc_bytes_vec_raw_parts() -> *mut BytesVecRawParts {
    let b = Box::new(BytesVecRawParts {
        ptr: MutPtr::from(ptr::null_mut()),
        len: 0,
        capacity: 0,
    });
    Box::leak(b)
}

#[no_mangle]
unsafe extern "C" fn dealloc_bytes_vec_raw_parts(ptr: *mut BytesVecRawParts) {
    let parts_ref = &*ptr;

    if parts_ref.capacity > 0 {
        let _: Vec<u8> = (*parts_ref).into_vec();
    }

    let b = Box::from_raw(ptr);
    drop(b);
}
