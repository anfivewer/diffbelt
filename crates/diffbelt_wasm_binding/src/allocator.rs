use crate::ptr::{NativePtrImpl, PtrImpl};
use alloc::vec::Vec;
use core::ptr;
use diffbelt_util_no_std::cast::{checked_positive_i32_to_usize, checked_usize_to_i32};

#[repr(transparent)]
pub struct BytesVecPtr {
    ptr: *mut u8,
}

#[repr(C)]
pub struct BytesVecFullPtr {
    ptr: *mut u8,
    capacity: i32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct BytesVecFull<P: PtrImpl = NativePtrImpl> {
    pub ptr: P::Ptr<u8>,
    pub len: i32,
    pub capacity: i32,
}

impl From<Vec<u8>> for BytesVecFullPtr {
    fn from(vec: Vec<u8>) -> Self {
        let capacity = vec.capacity();
        let capacity = checked_usize_to_i32(capacity);
        let ptr = vec.leak() as *mut [u8] as *mut u8;

        Self { ptr, capacity }
    }
}

impl BytesVecFull<NativePtrImpl> {
    pub fn null() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: -1,
            capacity: -1,
        }
    }

    pub unsafe fn into_vec(self) -> Vec<u8> {
        let Self { ptr, len, capacity } = self;

        let len = checked_positive_i32_to_usize(len);
        let capacity = checked_positive_i32_to_usize(capacity);

        Vec::from_raw_parts(ptr, len, capacity)
    }
}

pub trait VecAllocator {
    extern "C" fn alloc(len: i32) -> BytesVecPtr;
    unsafe extern "C" fn dealloc(ptr: BytesVecPtr, len: i32);
}
