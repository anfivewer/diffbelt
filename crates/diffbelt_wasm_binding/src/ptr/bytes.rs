use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use core::ptr;
use core::str::{from_utf8, Utf8Error};

use diffbelt_protos::{FlatbuffersType, OwnedSerialized};
use diffbelt_util_no_std::cast::{checked_positive_i32_to_usize, checked_usize_to_i32};

use crate::ptr::slice::SliceRawParts;
use crate::ptr::{NativePtrImpl, PtrImpl};

pub type BytesSlice<P = NativePtrImpl> = SliceRawParts<u8, P>;

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

impl<'fbb, T: FlatbuffersType<'fbb>> From<OwnedSerialized<'fbb, T>> for BytesVecRawParts {
    fn from(serialized: OwnedSerialized<'fbb, T>) -> Self {
        let buffer = serialized.into_vec();

        Self::from(buffer)
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

impl SliceRawParts<u8> {
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

#[no_mangle]
unsafe extern "C" fn ensure_vec_capacity(parts: *mut BytesVecRawParts, len: i32) {
    let mut vec = (&*parts).into_empty_vec();

    let len = checked_positive_i32_to_usize(len);

    if vec.capacity() < len {
        vec.reserve(len - vec.capacity());
    }

    unsafe { *parts = vec.into() };
}
