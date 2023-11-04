use core::ptr::slice_from_raw_parts;
use crate::ptr::{NativePtrImpl, PtrImpl};
use diffbelt_util_no_std::cast::{checked_positive_i32_to_usize, checked_usize_to_i32};

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct SliceRawParts<T: Clone, P: PtrImpl = NativePtrImpl> {
    pub ptr: P::Ptr<T>,
    pub len: i32,
}

impl<T: Clone> From<&[T]> for SliceRawParts<T> {
    fn from(value: &[T]) -> Self {
        Self {
            ptr: value as *const [T] as *const T,
            len: checked_usize_to_i32(value.len()),
        }
    }
}

impl <T: Clone> SliceRawParts<T> {
    pub unsafe fn as_slice(&self) -> &[T] {
        let Self { ptr, len } = *self;

        let slice = slice_from_raw_parts(ptr, checked_positive_i32_to_usize(len));
        let slice = &*slice;

        slice
    }
}