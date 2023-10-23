use flatbuffers::{
    FlatBufferBuilder, Follow, InvalidFlatbuffer, Push, Vector, Verifiable, WIPOffset,
};

pub mod protos;

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

    pub fn buffer_builder(&mut self) -> &mut FlatBufferBuilder<'fbb> {
        &mut self.buffer_builder_
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
}

pub struct Serialized<'a> {
    buffer_builder_: FlatBufferBuilder<'a>,
}

impl Serialized<'_> {
    pub fn data(&self) -> &[u8] {
        self.buffer_builder_.finished_data()
    }

    pub fn into_owned(self) -> OwnedSerialized {
        let (data, head) = self.buffer_builder_.collapse();

        OwnedSerialized { data_: data, head }
    }
}

pub struct OwnedSerialized {
    data_: Vec<u8>,
    head: usize,
}

impl OwnedSerialized {
    pub fn data(&self) -> &[u8] {
        &self.data_[self.head..]
    }
}
