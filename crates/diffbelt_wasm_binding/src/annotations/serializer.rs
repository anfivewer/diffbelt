use diffbelt_protos::{deserialize_unchecked, FlatbuffersType, OwnedSerialized, Serializer};
use diffbelt_util_no_std::comments::Annotated;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};

pub trait AsSerializerAnnotated<'fbb, F: FlatbuffersType<'fbb>> {
    unsafe fn as_serializer(&self) -> Serializer<'fbb, F>;
}

impl<'fbb, F: FlatbuffersType<'fbb>> AsSerializerAnnotated<'fbb, F>
    for Annotated<*mut BytesVecRawParts, F>
{
    unsafe fn as_serializer(&self) -> Serializer<'fbb, F> {
        let vec = (*self.value).into_empty_vec();
        Serializer::from_vec(vec)
    }
}

pub trait OwnedOutputAnnotated<'fbb, F: FlatbuffersType<'fbb>> {
    unsafe fn save_serialized(&self, serialized: OwnedSerialized<'fbb, F>);
}

impl<'fbb, F: FlatbuffersType<'fbb>> OwnedOutputAnnotated<'fbb, F>
    for Annotated<*mut BytesVecRawParts, F>
{
    unsafe fn save_serialized(&self, serialized: OwnedSerialized<'fbb, F>) {
        *self.value = serialized.into()
    }
}

pub trait InputAnnotated<'fbb, Input: FlatbuffersType<'fbb>> {
    unsafe fn deserialize(&self) -> Input::Inner;
}

impl<'fbb, Input: FlatbuffersType<'fbb>, Output> InputAnnotated<'fbb, Input>
    for Annotated<*mut BytesSlice, (Input, Output)>
{
    unsafe fn deserialize(&self) -> Input::Inner {
        let slice = unsafe { (&*self.value).as_slice() };

        let result = deserialize_unchecked::<Input>(slice);

        result
    }
}

pub trait RefOutputAnnotated<'fbb, F: FlatbuffersType<'fbb>> {
    unsafe fn save_owned_serialized(&self, serialized: &OwnedSerialized<'fbb, F>);
}

impl<'fbb, Input, Output: FlatbuffersType<'fbb>> RefOutputAnnotated<'fbb, Output>
    for Annotated<*mut BytesSlice, (Input, Output)>
{
    unsafe fn save_owned_serialized(&self, serialized: &OwnedSerialized<'fbb, Output>) {
        *self.value = serialized.as_bytes().into()
    }
}
