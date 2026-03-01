pub mod schedule_dealloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr;
use diffbelt_wasm_binding::ptr::bytes::VecRawParts;

use crate::ptr::bytes::{BytesSlice, BytesVecPtr, BytesVecRawParts, BytesVecWidePtr};
use crate::ptr::{ConstPtr, MutPtr};
use crate::requests::RequestId;

#[unsafe(no_mangle)]
unsafe extern "C" fn alloc(capacity: i32) -> BytesVecPtr {
    let vec = Vec::<u8>::with_capacity(capacity as usize);
    let ptr = vec.leak() as *mut [u8];
    BytesVecPtr {
        ptr: ptr as *mut u8,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dealloc(ptr: *mut u8, capacity: i32) {
    let ptr = BytesVecWidePtr {
        ptr,
        capacity,
    };

    unsafe {
        let _: Vec<u8> = ptr.into_empty_vec();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alloc_bytes_slice() -> *mut BytesSlice {
    let b = Box::new(BytesSlice {
        ptr: ConstPtr::from(ptr::null()),
        len: 0,
    });
    Box::leak(b)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dealloc_bytes_slice(ptr: *mut BytesSlice) {
    let b = Box::from_raw(ptr);
    drop(b);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dealloc_bytes_vec(ptr: i32, len: u32, capacity: u32) {
    let parts = BytesVecRawParts {
        ptr: MutPtr::new_i32(ptr),
        len,
        capacity,
    };

    if parts.capacity > 0 {
        let _: Vec<u8> = unsafe { parts.into_vec() };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alloc_bytes_vec_raw_parts() -> *mut BytesVecRawParts {
    let b = Box::new(BytesVecRawParts {
        ptr: MutPtr::from(ptr::null_mut()),
        len: 0,
        capacity: 0,
    });
    Box::leak(b)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dealloc_bytes_vec_raw_parts(ptr: *mut BytesVecRawParts) {
    let parts_ref = &*ptr;

    if parts_ref.capacity > 0 {
        let _: Vec<u8> = (*parts_ref).into_vec();
    }

    let b = Box::from_raw(ptr);
    drop(b);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alloc_vec_raw_parts_of_bytes_vec_raw_parts() -> *mut VecRawParts<BytesVecRawParts> {
    let b = Box::new(VecRawParts {
        ptr: MutPtr::from(ptr::null_mut()),
        len: 0,
        capacity: 0,
    });
    Box::leak(b)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dealloc_vec_raw_parts_of_bytes_vec_raw_parts(
    ptr: *mut VecRawParts<BytesVecRawParts>,
) {
    let parts_ref = &*ptr;

    if parts_ref.capacity > 0 {
        let _: Vec<BytesVecRawParts> = (*parts_ref).into_vec();
    }

    let b = Box::from_raw(ptr);
    drop(b);
}
