#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Write};
use core::marker::PhantomData;

use crate::align_util::{AlignedBytes, OwnedAlignedBytes};
use flatbuffers::{
    FlatBufferBuilder, Follow, ForwardsUOffset, Push, Verifiable, Verifier, VerifierOptions,
};
pub use flatbuffers::{InvalidFlatbuffer, Vector, WIPOffset};

use crate::error::{FlatbufferError, InvalidFlatbufferWithBuffer};

extern crate self as diffbelt_protos;

pub mod align_util;
pub mod error;
pub mod protos;
#[cfg(test)]
mod tests;

pub const FLATBUFFERS_ALIGNMENT: usize = 8;

trait FlatbuffersType<'fbb>: Follow<'fbb> + Verifiable + 'fbb {}

impl<'fbb, T: Follow<'fbb> + Verifiable + 'fbb> FlatbuffersType<'fbb> for T {}

pub trait FlatbuffersGenericType {
    type FlatType<'a>: FlatbuffersType<'a> + Debug;
    fn name() -> &'static str;
}

pub fn deserialize<'a, T: FlatbuffersGenericType>(
    bytes: AlignedBytes<'a, FLATBUFFERS_ALIGNMENT>,
) -> Result<<T::FlatType<'a> as Follow>::Inner, InvalidFlatbuffer> {
    flatbuffers::root::<T::FlatType<'a>>(bytes.as_slice())
}

pub struct Serializer<'a, T: FlatbuffersGenericType> {
    buffer_builder_: FlatBufferBuilder<'a>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T: FlatbuffersGenericType> Debug for Serializer<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("Serializer(")?;
        f.write_str(T::name())?;
        f.write_str(")")?;
        Ok(())
    }
}

impl<'fbb, F: FlatbuffersGenericType> Serializer<'fbb, F> {
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

    pub fn start_vector<T: Push>(&mut self, items_count: usize) {
        self.buffer_builder_.start_vector::<T>(items_count);
    }

    pub fn push<T: Push>(&mut self, item: T) {
        self.buffer_builder_.push(item);
    }

    pub fn end_vector<T: Push>(&mut self, items_count: usize) -> WIPOffset<Vector<'fbb, T>> {
        self.buffer_builder_.end_vector(items_count)
    }

    pub fn finish(mut self, root: WIPOffset<F::FlatType<'fbb>>) -> OwnedSerialized<F> {
        let () = self.buffer_builder_.finish_minimal(root);
        let len = self.buffer_builder_.finished_data().len();
        let (buffer, head) = self.buffer_builder_.collapse();

        let bytes = OwnedAlignedBytes::new(buffer, head, len).expect("serialization error");

        OwnedSerialized {
            bytes,
            phantom: PhantomData::default(),
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer_builder_.unfinished_data().len()
    }

    pub fn into_vec(self) -> Vec<u8> {
        let (buffer, _) = self.buffer_builder_.collapse();
        buffer
    }
}

pub struct OwnedSerialized<T: FlatbuffersGenericType> {
    bytes: OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>,
    phantom: PhantomData<T>,
}

impl<T: FlatbuffersGenericType> Debug for OwnedSerialized<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("OwnedSerialized(?)")
    }
}

impl<T: FlatbuffersGenericType> PartialEq for OwnedSerialized<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.as_slice() == other.bytes.as_slice()
    }
}

impl<T: FlatbuffersGenericType> Eq for OwnedSerialized<T> {}

impl<F: FlatbuffersGenericType> OwnedSerialized<F> {
    pub fn from_aligned_bytes(
        bytes: OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>,
    ) -> Result<Self, FlatbufferError> {
        let opts = VerifierOptions::default();
        let mut v = Verifier::new(&opts, bytes.as_slice());
        let () = match <ForwardsUOffset<F::FlatType<'_>>>::run_verifier(&mut v, 0) {
            Ok(()) => (),
            Err(error) => {
                return Err(FlatbufferError::InvalidFlatbufferWithBuffer(
                    InvalidFlatbufferWithBuffer {
                        buffer: Some(bytes.into_vec()),
                        error,
                    },
                ));
            }
        };

        Ok(Self {
            bytes,
            phantom: PhantomData::default(),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn data(&self) -> <F::FlatType<'_> as Follow>::Inner {
        unsafe { flatbuffers::root_unchecked::<F::FlatType<'_>>(self.as_bytes()) }
    }

    pub fn into_aligned_bytes(self) -> OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT> {
        self.bytes
    }

    pub fn into_buffer(self) -> Vec<u8> {
        self.bytes.into_vec()
    }

    #[deprecated]
    pub fn into_buffer_vec(self) -> Vec<u8> {
        self.bytes.into_vec()
    }

    #[deprecated]
    pub fn into_owned(self) -> Self {
        self
    }

    pub fn into_raw_parts(self) -> SerializedRawParts {
        let (buffer, head, len) = self.bytes.into_raw_parts();

        SerializedRawParts { buffer, head, len }
    }
}

pub struct SerializedRawParts {
    pub buffer: Vec<u8>,
    pub head: usize,
    pub len: usize,
}

#[deprecated]
pub type Serialized<T: FlatbuffersGenericType> = OwnedSerialized<T>;
