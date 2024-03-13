use crate::ptr::{ConstPtr, NativePtrImpl, PtrImpl};
use bytemuck::{Pod, Zeroable};
use core::ptr::slice_from_raw_parts;
use diffbelt_util_no_std::cast::{checked_positive_i32_to_usize, checked_usize_to_i32};

#[derive(Pod, Zeroable, Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct SliceRawParts<T: Pod, P: PtrImpl = NativePtrImpl> {
    pub ptr: P::Ptr<T>,
    pub len: i32,
}

impl<T: Pod> From<&[T]> for SliceRawParts<T> {
    fn from(value: &[T]) -> Self {
        Self {
            ptr: ConstPtr::from(value as *const [T] as *const T),
            len: checked_usize_to_i32(value.len()),
        }
    }
}

impl<T: Pod> SliceRawParts<T> {
    pub unsafe fn as_slice(&self) -> &[T] {
        let Self { ptr, len } = *self;

        let slice = slice_from_raw_parts(ptr.as_ptr(), checked_positive_i32_to_usize(len));
        let slice = &*slice;

        slice
    }
}
