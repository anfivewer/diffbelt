use crate::annotations::{Annotated, AnnotatedTrait, FlatbufferAnnotated, InputOutputAnnotated};
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_protos::{
    deserialize_unchecked, FlatbuffersType, OwnedSerialized, Serializer, WIPOffset,
};

pub struct SerializerFromAnnotated<'fbb, F: FlatbuffersType<'fbb>, A: AnnotatedTrait> {
    original: A,
    serializer: Serializer<'fbb, F>,
}

impl<'fbb, F: FlatbuffersType<'fbb>, A: AnnotatedTrait> SerializerFromAnnotated<'fbb, F, A> {
    pub fn serializer_mut(&mut self) -> &mut Serializer<'fbb, F> {
        &mut self.serializer
    }
}

impl<'fbb, F: FlatbuffersType<'fbb>, A: AnnotatedTrait<Value = *mut BytesVecRawParts>>
    SerializerFromAnnotated<'fbb, F, A>
{
    pub fn finish(self, root: WIPOffset<F>) -> SerializedWithAnnotated<'fbb, F, A> {
        let serialized = self.serializer.finish(root).into_owned();

        SerializedWithAnnotated {
            original: self.original,
            serialized,
        }
    }
}

pub struct SerializedWithAnnotated<'fbb, F: FlatbuffersType<'fbb>, A> {
    original: A,
    serialized: OwnedSerialized<'fbb, F>,
}

impl<'fbb, F: FlatbuffersType<'fbb>, A: AnnotatedTrait<Value = *mut BytesVecRawParts>>
    SerializedWithAnnotated<'fbb, F, A>
{
    pub unsafe fn save(self) {
        **self.original.value() = self.serialized.into()
    }
}

impl<'fbb, F: FlatbuffersType<'fbb>, A: AnnotatedTrait> SerializedWithAnnotated<'fbb, F, A> {
    pub fn serialized_data(&self) -> FlatbufferAnnotated<&[u8], F> {
        FlatbufferAnnotated::from(self.serialized.as_bytes())
    }
}

pub trait IntoSerializerAnnotated<'fbb, F: FlatbuffersType<'fbb>>: Sized + AnnotatedTrait {
    unsafe fn into_serializer(self) -> SerializerFromAnnotated<'fbb, F, Self>;
}

impl<'fbb, F: FlatbuffersType<'fbb>> IntoSerializerAnnotated<'fbb, F>
    for FlatbufferAnnotated<*mut BytesVecRawParts, F>
{
    unsafe fn into_serializer(self) -> SerializerFromAnnotated<'fbb, F, Self> {
        let vec = (*self.value).into_empty_vec();
        let serializer = Serializer::from_vec(vec);

        SerializerFromAnnotated {
            original: self,
            serializer,
        }
    }
}

pub trait InputAnnotated<'fbb, Input: FlatbuffersType<'fbb>> {
    unsafe fn deserialize(&self) -> Input::Inner;
}

impl<'fbb, Input: FlatbuffersType<'fbb>, Output> InputAnnotated<'fbb, Input>
    for FlatbufferAnnotated<*mut BytesSlice, (Input, Output)>
{
    unsafe fn deserialize(&self) -> Input::Inner {
        let slice = unsafe { (&*self.value).as_slice() };

        let result = deserialize_unchecked::<Input>(slice);

        result
    }
}

impl<'fbb, Input: FlatbuffersType<'fbb>, Output> InputAnnotated<'fbb, Input>
    for InputOutputAnnotated<*mut BytesSlice, Input, Output>
{
    unsafe fn deserialize(&self) -> Input::Inner {
        let slice = unsafe { (&*self.value).as_slice() };

        let result = deserialize_unchecked::<Input>(slice);

        result
    }
}

pub trait OutputAnnotated<'fbb, Output: FlatbuffersType<'fbb>, AValue> {
    unsafe fn save<A: AnnotatedTrait<Value = AValue, Annotation = Output>>(&self, data: A);
}

impl<'fbb, Input: FlatbuffersType<'fbb>, Output: FlatbuffersType<'fbb>>
    OutputAnnotated<'fbb, Output, &'fbb [u8]>
    for InputOutputAnnotated<*mut BytesSlice, Input, Output>
{
    unsafe fn save<A: AnnotatedTrait<Value = &'fbb [u8], Annotation = Output>>(&self, data: A) {
        let data = *data.value();
        *self.value = data.into();
    }
}
