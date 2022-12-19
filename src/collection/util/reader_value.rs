use crate::common::{GenerationId, IsByteArray};
use std::str::from_utf8;

#[derive(Debug)]
pub struct OwnedReaderValue(Box<[u8]>);
pub struct ReaderValue<'a>(&'a [u8]);

impl OwnedReaderValue {
    pub fn new(
        collection_id: Option<&str>,
        generation_id: Option<GenerationId<'_>>,
    ) -> Result<Self, ()> {
        let collection_id: &[u8] = collection_id.map(|name| name.as_bytes()).unwrap_or(b"");
        let generation_id: &[u8] = generation_id
            .as_ref()
            .map(|gen| gen.get_byte_array())
            .unwrap_or(b"");

        if collection_id.len() > 255 || generation_id.len() > 255 {
            return Err(());
        }

        let mut value = Vec::with_capacity(2 + collection_id.len() + generation_id.len());

        value.push(collection_id.len() as u8);
        value.extend_from_slice(collection_id);
        value.push(generation_id.len() as u8);
        value.extend_from_slice(generation_id);

        Ok(OwnedReaderValue(value.into_boxed_slice()))
    }

    pub fn from_vec(bytes: Vec<u8>) -> Result<Self, ()> {
        let result = ReaderValue::from_slice(&bytes)?;
        Ok(result.to_owned())
    }

    pub fn as_ref(&self) -> ReaderValue<'_> {
        ReaderValue(&self.0)
    }
}

impl<'a> ReaderValue<'a> {
    pub fn get_collection_id(&self) -> &str {
        let collection_id_len = self.0[0] as usize;

        let bytes = &self.0[1..(1 + collection_id_len)];

        std::str::from_utf8(bytes).unwrap()
    }

    pub fn get_generation_id(&self) -> GenerationId<'a> {
        let collection_id_len = self.0[0] as usize;

        let bytes = &self.0[(2 + collection_id_len)..];

        GenerationId(bytes)
    }

    pub fn from_slice(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() < 2 {
            return Err(());
        }

        let collection_id_len = bytes[0] as usize;

        if bytes.len() < 2 + collection_id_len {
            return Err(());
        }

        let utf8_validation = from_utf8(&bytes[1..(1 + collection_id_len)]);
        match utf8_validation {
            Err(_) => {
                return Err(());
            }
            _ => {}
        }

        let generation_id_len = bytes[1 + collection_id_len] as usize;

        if bytes.len() != 2 + collection_id_len + generation_id_len {
            return Err(());
        }

        Ok(ReaderValue(bytes))
    }

    pub fn to_owned(&self) -> OwnedReaderValue {
        OwnedReaderValue(self.0.into())
    }
}

impl IsByteArray for OwnedReaderValue {
    fn get_byte_array(&self) -> &[u8] {
        &self.0
    }
}
