#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;
use flatbuffers::{FlatBufferBuilder, Follow, Push, Verifiable};
pub use flatbuffers::{InvalidFlatbuffer, Vector, WIPOffset};

pub mod protos;
pub mod util;

pub trait FlatbuffersType<'fbb>: Follow<'fbb> + Verifiable + 'fbb {}
impl <'fbb, T: Follow<'fbb> + Verifiable + 'fbb> FlatbuffersType<'fbb> for T {}

pub fn deserialize<'fbb, T: FlatbuffersType<'fbb>>(
    bytes: &'fbb [u8],
) -> Result<T::Inner, InvalidFlatbuffer> {
    flatbuffers::root::<T>(bytes)
}

pub struct Serializer<'fbb, T: FlatbuffersType<'fbb>> {
    buffer_builder_: FlatBufferBuilder<'fbb>,
    phantom: PhantomData<T>,
}

impl<'fbb, F: FlatbuffersType<'fbb>> Serializer<'fbb, F> {
    pub fn new() -> Self {
        Self {
            buffer_builder_: FlatBufferBuilder::new(),
            phantom: PhantomData::default(),
        }
    }

    pub fn from_vec(mut buffer: Vec<u8>) -> Self {
        buffer.clear();

        Self {
            buffer_builder_: FlatBufferBuilder::from_vec(buffer),
            phantom: PhantomData::default(),
        }
    }

    pub fn buffer_builder(&mut self) -> &mut FlatBufferBuilder<'fbb> {
        &mut self.buffer_builder_
    }

    pub fn create_string(&mut self, value: &str) -> WIPOffset<&'fbb str> {
        self.buffer_builder_.create_string(value)
    }

    pub fn create_vector<'b, T: Push + 'b>(
        &mut self,
        items: &'b [T],
    ) -> WIPOffset<Vector<'fbb, T::Output>> {
        self.buffer_builder_.create_vector(items)
    }

    pub fn finish(mut self, root: WIPOffset<F>) -> Serialized<'fbb, F> {
        () = self.buffer_builder_.finish_minimal(root);

        Serialized {
            buffer_builder_: self.buffer_builder_,
            phantom: PhantomData::default(),
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        let (buffer, _) = self.buffer_builder_.collapse();
        buffer
    }
}

pub struct Serialized<'fbb, F: FlatbuffersType<'fbb>> {
    buffer_builder_: FlatBufferBuilder<'fbb>,
    phantom: PhantomData<F>,
}

impl <'fbb, F: FlatbuffersType<'fbb>> Serialized<'fbb, F> {
    pub fn as_bytes(&self) -> &[u8] {
        self.buffer_builder_.finished_data()
    }

    pub fn data(&'fbb self) -> F::Inner {
        unsafe { flatbuffers::root_unchecked::<F>(self.as_bytes()) }
    }

    pub fn into_owned(self) -> OwnedSerialized<'fbb, F> {
        let len = self.buffer_builder_.finished_data().len();
        let (data, head) = self.buffer_builder_.collapse();

        OwnedSerialized {
            buffer: data,
            head,
            len,
            phantom: PhantomData::default(),
        }
    }

    pub fn into_empty_vec(self) -> Vec<u8> {
        let (buffer, _) = self.buffer_builder_.collapse();
        buffer
    }
}

pub struct OwnedSerialized<'fbb, T: FlatbuffersType<'fbb>> {
    buffer: Vec<u8>,
    head: usize,
    len: usize,
    phantom: PhantomData<&'fbb T>,
}

impl <'fbb, F: FlatbuffersType<'fbb>> OwnedSerialized<'fbb, F> {
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[self.head..(self.head + self.len)]
    }

    pub fn data(&'fbb self) -> F::Inner {
        unsafe { flatbuffers::root_unchecked::<F>(self.as_bytes()) }
    }

    pub fn into_raw_parts(self) -> SerializedRawParts {
        SerializedRawParts {
            buffer: self.buffer,
            head: self.head,
            len: self.len,
        }
    }
}

pub struct SerializedRawParts {
    pub buffer: Vec<u8>,
    pub head: usize,
    pub len: usize,
}
