use alloc::vec::Vec;
use enum_as_inner::EnumAsInner;
use flatbuffers::InvalidFlatbuffer;
use thiserror_no_std::Error;

#[derive(Error, EnumAsInner, Debug)]
pub enum FlatbufferError {
    InvalidFlatbuffer(InvalidFlatbuffer),
    InvalidFlatbufferWithBuffer(InvalidFlatbufferWithBuffer),
}

#[derive(Debug)]
pub struct InvalidFlatbufferWithBuffer {
    pub buffer: Option<Vec<u8>>,
    pub error: InvalidFlatbuffer,
}

pub fn map_flatbuffer_error_to_return_buffer(
    buffer_holder: &mut Option<Vec<u8>>,
) -> impl FnOnce(FlatbufferError) -> FlatbufferError + '_ {
    |err| match err.into_invalid_flatbuffer_with_buffer() {
        Ok(InvalidFlatbufferWithBuffer { buffer, error }) => {
            *buffer_holder = buffer;

            FlatbufferError::InvalidFlatbuffer(error)
        }
        Err(err) => err,
    }
}
