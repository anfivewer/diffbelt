use diffbelt_protos::{FlatbuffersGenericType, OwnedSerialized, Serializer, WIPOffset};

use crate::annotations::{Annotated, AnnotatedTrait, FlatbufferAnnotated, InputOutputAnnotated};
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};

pub struct SerializerFromAnnotated<'a, F: FlatbuffersGenericType, A: AnnotatedTrait> {
    original: A,
    serializer: Serializer<'a, F>,
}

impl<'a, F: FlatbuffersGenericType, A: AnnotatedTrait> SerializerFromAnnotated<'a, F, A> {
    pub fn serializer_mut(&mut self) -> &mut Serializer<'a, F> {
        &mut self.serializer
    }
}

impl<'a, F: FlatbuffersGenericType, A: AnnotatedTrait<Value = *mut BytesVecRawParts>>
    SerializerFromAnnotated<'a, F, A>
{
    pub fn finish(self, root: WIPOffset<F::FlatType<'a>>) -> SerializedWithAnnotated<F, A> {
        let serialized = self.serializer.finish(root);

        SerializedWithAnnotated {
            original: self.original,
            serialized,
        }
    }
}

pub struct SerializedWithAnnotated<F: FlatbuffersGenericType, A> {
    original: A,
    serialized: OwnedSerialized<F>,
}

impl<F: FlatbuffersGenericType, A: AnnotatedTrait<Value = *mut BytesVecRawParts>>
    SerializedWithAnnotated<F, A>
{
    pub unsafe fn save(self) {
        **self.original.value() = self.serialized.into();
    }
}

impl<F: FlatbuffersGenericType, A: AnnotatedTrait> SerializedWithAnnotated<F, A> {
    pub fn serialized_data(&self) -> FlatbufferAnnotated<&[u8], F> {
        FlatbufferAnnotated::from(self.serialized.as_bytes())
    }
}

pub trait IntoSerializerAnnotated<'a, F: FlatbuffersGenericType>: Sized + AnnotatedTrait {
    unsafe fn into_serializer(self) -> SerializerFromAnnotated<'a, F, Self>;
}

impl<'a, F: FlatbuffersGenericType> IntoSerializerAnnotated<'a, F>
    for FlatbufferAnnotated<*mut BytesVecRawParts, F>
{
    unsafe fn into_serializer(self) -> SerializerFromAnnotated<'a, F, Self> {
        let vec = (*self.value).into_empty_vec();
        let serializer = Serializer::from_vec(vec);

        SerializerFromAnnotated {
            original: self,
            serializer,
        }
    }
}

pub trait OutputAnnotated<'a, Output: FlatbuffersGenericType, AValue> {
    unsafe fn save<A: AnnotatedTrait<Value = AValue, Annotation = Output>>(&self, data: A);
}

impl<'a, Input: FlatbuffersGenericType, Output: FlatbuffersGenericType>
    OutputAnnotated<'a, Output, &'a [u8]> for InputOutputAnnotated<*mut BytesSlice, Input, Output>
{
    unsafe fn save<A: AnnotatedTrait<Value = &'a [u8], Annotation = Output>>(&self, data: A) {
        let data = *data.value();
        *self.value = data.into();
    }
}
