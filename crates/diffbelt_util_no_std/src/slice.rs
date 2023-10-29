use core::mem::size_of;
use thiserror_no_std::Error;

use crate::cast::{checked_usize_to_isize, ptr_to_usize, unchecked_isize_to_usize};

#[derive(Error, Debug)]
pub enum SliceOffsetError {
    SubSliceIsNotPartOfOrigin,
    SubSliceOverflows,
    Unaligned,
}

pub fn get_slice_offset_in_other_slice<T>(
    origin: &[T],
    sub_slice: &[T],
) -> Result<usize, SliceOffsetError> {
    let origin_ptr = origin as *const [T] as *const T;
    let sub_slice_ptr = sub_slice as *const [T] as *const T;

    let origin_ptr = checked_usize_to_isize(ptr_to_usize(origin_ptr));
    let sub_slice_ptr = checked_usize_to_isize(ptr_to_usize(sub_slice_ptr));

    let diff = sub_slice_ptr - origin_ptr;

    if diff < 0 {
        return Err(SliceOffsetError::SubSliceIsNotPartOfOrigin);
    }

    let diff = unchecked_isize_to_usize(diff);

    let offset = diff / size_of::<T>();
    let rem = diff % size_of::<T>();

    if rem != 0 {
        return Err(SliceOffsetError::Unaligned);
    }

    if offset + sub_slice.len() > origin.len() {
        return Err(SliceOffsetError::SubSliceOverflows);
    }

    Ok(offset)
}
