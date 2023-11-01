#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use flatbuffers::{FlatBufferBuilder, Follow, Push, Verifiable};
pub use flatbuffers::{InvalidFlatbuffer, WIPOffset, Vector};

pub mod protos;
pub mod util;

pub fn deserialize<'buf, T: 'buf + Follow<'buf> + Verifiable>(
    bytes: &'buf [u8],
) -> Result<T::Inner, InvalidFlatbuffer> {
    flatbuffers::root::<T>(bytes)
}

pub struct Serializer<'a> {
    buffer_builder_: FlatBufferBuilder<'a>,
}

impl<'fbb> Serializer<'fbb> {
    pub fn new() -> Self {
        Self {
            buffer_builder_: FlatBufferBuilder::new(),
        }
    }

    pub fn from_vec(mut buffer: Vec<u8>) -> Self {
        buffer.clear();

        Self {
            buffer_builder_: FlatBufferBuilder::from_vec(buffer),
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

    pub fn finish<T>(mut self, root: WIPOffset<T>) -> Serialized<'fbb> {
        () = self.buffer_builder_.finish_minimal(root);

        Serialized {
            buffer_builder_: self.buffer_builder_,
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        let (buffer, _) = self.buffer_builder_.collapse();
        buffer
    }
}

pub struct Serialized<'a> {
    buffer_builder_: FlatBufferBuilder<'a>,
}

impl Serialized<'_> {
    pub fn data(&self) -> &[u8] {
        self.buffer_builder_.finished_data()
    }

    pub fn into_owned(self) -> OwnedSerialized {
        let len = self.buffer_builder_.finished_data().len();
        let (data, head) = self.buffer_builder_.collapse();

        OwnedSerialized {
            buffer: data,
            head,
            len,
        }
    }

    pub fn into_empty_vec(self) -> Vec<u8> {
        let (buffer, _) = self.buffer_builder_.collapse();
        buffer
    }
}

pub struct OwnedSerialized {
    pub buffer: Vec<u8>,
    pub head: usize,
    pub len: usize,
}

impl OwnedSerialized {
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[self.head..(self.head + self.len)]
    }
}
