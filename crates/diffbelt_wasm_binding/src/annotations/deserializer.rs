use crate::annotations::Annotated;
use crate::annotations::head_len_vec::FlatbuffersHeadLenVecAnnotation;
use crate::debug_print_string;
use crate::ptr::bytes::{BytesVecRawParts, VecRawParts};
use core::slice;
use diffbelt_protos::align_util::{AlignedBytes, AlignedBytesError};
use diffbelt_protos::{
    FLATBUFFERS_ALIGNMENT, FlatbuffersGenericType, FlatbuffersType, Follow, InvalidFlatbuffer,
    deserialize,
};
use diffbelt_util_no_std::bytes::{read_u32_be, write_u32_be};
use diffbelt_util_no_std::cast::{
    checked_positive_isize_to_usize, checked_usize_to_u32, u32_to_usize,
};
use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum FlatbuffersDeserializeError {
    #[error("{0:?}")]
    AlignedBytes(#[from] AlignedBytesError),
    #[error("{0:?}")]
    Flatbuffer(#[from] InvalidFlatbuffer),
}

impl<T: FlatbuffersGenericType>
    Annotated<*mut BytesVecRawParts, FlatbuffersHeadLenVecAnnotation<T>>
{
    pub unsafe fn deserialize(
        &mut self,
    ) -> Result<<T::FlatType<'_> as Follow>::Inner, FlatbuffersDeserializeError> {
        // SAFETY: called must be sure, that pointer is valid
        let mut buffer = unsafe { (&*self.value).into_vec() };
        assert!(buffer.len() >= 8, "buffer should hold two u32");

        let head = u32_to_usize(read_u32_be(&buffer[0..4]));
        let len = u32_to_usize(read_u32_be(&buffer[4..8]));

        let aligned = AlignedBytes::<FLATBUFFERS_ALIGNMENT>::align_in_prefixed_vec(
            &mut buffer,
            8,
            head,
            len,
        )?;
        let new_head_ptr = aligned.as_slice().as_ptr();

        // SAFETY: trust in called
        unsafe { *self.value = VecRawParts::from(buffer) };
        // SAFETY: just written
        let raw_parts = unsafe { &*self.value };

        // SAFETY: head should be greater than start of buffer
        let head = unsafe { new_head_ptr.offset_from(raw_parts.ptr.as_ptr()) };
        let head = checked_positive_isize_to_usize(head);
        let head_u32 = checked_usize_to_u32(head);

        // SAFETY: buffer is just stored and len greater than 8
        let buffer_slice = unsafe {
            slice::from_raw_parts_mut(raw_parts.ptr.as_mut_ptr(), u32_to_usize(raw_parts.len))
        };

        write_u32_be(&mut buffer_slice[0..4], head_u32);

        let aligned = AlignedBytes::ensure_alignment(&buffer_slice[head..(head + len)])?;

        let result = deserialize::<T>(aligned)?;

        Ok(result)
    }
}
