use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use core::ptr;
use core::ptr::slice_from_raw_parts;
use core::str::{from_utf8, Utf8Error};

use bytemuck::{Pod, Zeroable};

use diffbelt_protos::{FlatbuffersType, OwnedSerialized};
use diffbelt_util_no_std::cast::{checked_positive_i32_to_usize, checked_usize_to_i32};

use crate::ptr::slice::SliceRawParts;
use crate::ptr::{ConstPtr, MutPtr, NativePtrImpl, PtrImpl};

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
pub struct VecRawParts<T: Pod, P: PtrImpl = NativePtrImpl> {
    pub ptr: P::MutPtr<T>,
    pub len: i32,
    pub capacity: i32,
}

unsafe impl<T: Pod, P: PtrImpl> Zeroable for VecRawParts<T, P> {}
unsafe impl<T: Pod, P: PtrImpl + Copy + 'static> Pod for VecRawParts<T, P> {}

pub type BytesVecRawParts<P = NativePtrImpl> = VecRawParts<u8, P>;

impl<T: Pod> VecRawParts<T, NativePtrImpl> {
    pub unsafe fn into_empty_vec(self) -> Vec<T> {
        let ptr = self.ptr.as_mut_ptr();
        Vec::from_raw_parts(ptr, 0, self.capacity as usize)
    }
}

impl<'fbb, T: FlatbuffersType<'fbb>> From<OwnedSerialized<'fbb, T>> for BytesVecRawParts {
    fn from(serialized: OwnedSerialized<'fbb, T>) -> Self {
        let buffer = serialized.into_buffer_vec();

        Self::from(buffer)
    }
}

impl BytesVecWidePtr {
    pub unsafe fn into_empty_vec(self) -> Vec<u8> {
        Vec::from_raw_parts(self.ptr, 0, self.capacity as usize)
    }
}

impl<T: Pod> From<Vec<T>> for VecRawParts<T> {
    fn from(vec: Vec<T>) -> Self {
        let len = vec.len();
        let len = checked_usize_to_i32(len);
        let capacity = vec.capacity();
        let capacity = checked_usize_to_i32(capacity);
        let ptr = vec.leak() as *mut [T] as *mut T;

        Self {
            ptr: MutPtr::from(ptr),
            len,
            capacity,
        }
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

impl<T: Pod> From<&VecRawParts<T>> for SliceRawParts<T> {
    fn from(value: &VecRawParts<T>) -> Self {
        Self {
            ptr: ConstPtr::from(value.ptr),
            len: value.len,
        }
    }
}

impl<T: Pod> VecRawParts<T> {
    pub fn null() -> Self {
        Self {
            ptr: MutPtr::from(ptr::null_mut()),
            len: -1,
            capacity: -1,
        }
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        let Self {
            ptr,
            len,
            capacity: _,
        } = self;

        let slice = slice_from_raw_parts(ptr.as_ptr(), checked_positive_i32_to_usize(*len));
        let slice = &*slice;

        slice
    }

    pub unsafe fn into_vec(self) -> Vec<T> {
        let Self { ptr, len, capacity } = self;

        let len = checked_positive_i32_to_usize(len);
        let capacity = checked_positive_i32_to_usize(capacity);

        Vec::from_raw_parts(ptr.as_mut_ptr(), len, capacity)
    }
}

impl VecRawParts<u8> {
    pub unsafe fn into_string(self) -> Result<String, FromUtf8Error> {
        let vec = self.into_vec();
        String::from_utf8(vec)
    }
}

#[no_mangle]
unsafe extern "C" fn ensure_vec_capacity(parts: *mut BytesVecRawParts, len: i32) {
    let mut vec = (&*parts).into_vec();

    let len = checked_positive_i32_to_usize(len);

    if vec.capacity() < len {
        vec.reserve(len - vec.len());
    }

    unsafe { *parts = vec.into() };
}

#[no_mangle]
unsafe extern "C" fn ensure_vec_of_bytes_vec_raw_parts_capacity(
    parts: *mut VecRawParts<BytesVecRawParts>,
    len: i32,
) {
    let mut vec = (&*parts).into_vec();

    let len = checked_positive_i32_to_usize(len);

    if vec.capacity() < len {
        vec.reserve(len - vec.len());
    }

    unsafe { *parts = vec.into() };
}
