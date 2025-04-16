use crate::annotations::Annotated;
use crate::annotations::deserializer::FlatbuffersDeserializeError;
use crate::ptr::bytes::BytesVecRawParts;
use core::marker::PhantomData;
use diffbelt_protos::{FlatbuffersGenericType, Follow, Serialized};
use diffbelt_util_no_std::bytes::push_u32_be_to_vec;
use diffbelt_util_no_std::cast::checked_usize_to_u32;

pub struct FlatbuffersHeadLenVecAnnotation<T: FlatbuffersGenericType> {
    phantom: PhantomData<T>,
}

impl<T: FlatbuffersGenericType>
    Annotated<*mut BytesVecRawParts, FlatbuffersHeadLenVecAnnotation<T>>
{
    pub unsafe fn save(&self, serialized: Serialized<T>) {
        let mut buffer = unsafe { (&*self.value).into_empty_vec() };
        let serialized = serialized.as_bytes();

        buffer.reserve(4 + 4 + serialized.len());

        push_u32_be_to_vec(&mut buffer, 8);
        push_u32_be_to_vec(&mut buffer, checked_usize_to_u32(serialized.len()));

        buffer.extend_from_slice(serialized);

        unsafe {
            *self.value = buffer.into();
        }
    }
}
