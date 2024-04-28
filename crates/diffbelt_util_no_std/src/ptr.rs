use crate::cast::{ptr_as_usize, unchecked_isize_to_usize};

pub fn relative_slice_location<T>(origin: &[T], slice: &[T]) -> Option<usize> {
    let origin_ptr = origin.as_ptr();
    let slice_ptr = slice.as_ptr();

    let origin_ptr_n = ptr_as_usize(origin_ptr);
    let slice_ptr_n = ptr_as_usize(slice_ptr);

    if slice_ptr_n < origin_ptr_n || slice_ptr_n > origin_ptr_n + origin.len() {
        return None;
    }

    return unsafe { Some(unchecked_isize_to_usize(slice_ptr.offset_from(origin_ptr))) };
}
