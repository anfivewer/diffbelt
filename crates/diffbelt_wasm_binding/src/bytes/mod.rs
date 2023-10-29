use crate::ptr::{NativePtrImpl, PtrImpl};
use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use core::ptr;
use core::ptr::slice_from_raw_parts;
use core::str::{from_utf8, Utf8Error};
use diffbelt_util_no_std::cast::{checked_positive_i32_to_usize, checked_usize_to_i32};

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct BytesSlice<P: PtrImpl = NativePtrImpl> {
    pub ptr: P::Ptr<u8>,
    pub len: i32,
}

#[repr(transparent)]
pub struct BytesVecPtr {
    pub ptr: *mut u8,
}

#[repr(C)]
pub struct BytesVecWidePtr {
    pub ptr: *mut u8,
    pub capacity: i32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct BytesVecRawParts<P: PtrImpl = NativePtrImpl> {
    pub ptr: P::MutPtr<u8>,
    pub len: i32,
    pub capacity: i32,
}

impl BytesVecRawParts<NativePtrImpl> {
    pub unsafe fn into_empty_vec(self) -> Vec<u8> {
        Vec::from_raw_parts(self.ptr, 0, self.capacity as usize)
    }
}

impl BytesVecWidePtr {
    pub unsafe fn into_empty_vec(self) -> Vec<u8> {
        Vec::from_raw_parts(self.ptr, 0, self.capacity as usize)
    }
}

impl From<Vec<u8>> for BytesVecRawParts {
    fn from(vec: Vec<u8>) -> Self {
        let len = vec.len();
        let len = checked_usize_to_i32(len);
        let capacity = vec.capacity();
        let capacity = checked_usize_to_i32(capacity);
        let ptr = vec.leak() as *mut [u8] as *mut u8;

        Self { ptr, len, capacity }
    }
}

impl From<Vec<u8>> for BytesVecWidePtr {
    fn from(vec: Vec<u8>) -> Self {
        let capacity = vec.capacity();
        let capacity = checked_usize_to_i32(capacity);
        let ptr = vec.leak() as *mut [u8] as *mut u8;

        Self { ptr, capacity }
    }
}

impl BytesSlice {
    pub unsafe fn as_slice(&self) -> &[u8] {
        let Self { ptr, len } = *self;

        let slice = slice_from_raw_parts(ptr, len as usize);
        let slice = &*slice as &[u8];

        slice
    }

    pub unsafe fn as_str(&self) -> Result<&str, Utf8Error> {
        from_utf8(self.as_slice())
    }
}

impl BytesVecRawParts {
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

    pub unsafe fn into_string(self) -> Result<String, FromUtf8Error> {
        let vec = self.into_vec();
        String::from_utf8(vec)
    }
}
