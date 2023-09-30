use crate::common::{GenerationId, IsByteArray};
use std::str::from_utf8;
use diffbelt_util::cast::u8_to_usize;

#[derive(Debug)]
pub struct OwnedReaderValue(Box<[u8]>);
pub struct ReaderValue<'a>(&'a [u8]);

impl OwnedReaderValue {
    pub fn new(collection_name: Option<&str>, generation_id: GenerationId<'_>) -> Result<Self, ()> {
        let collection_name: &[u8] = collection_name.map(|name| name.as_bytes()).unwrap_or(b"");
        let generation_id: &[u8] = generation_id.get_byte_array();

        if collection_name.len() > 255 || generation_id.len() > 255 {
            return Err(());
        }

        let mut value = Vec::with_capacity(2 + collection_name.len() + generation_id.len());

        value.push(collection_name.len() as u8);
        value.extend_from_slice(collection_name);
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
    pub fn get_collection_name(&self) -> &str {
        let collection_name_len = u8_to_usize(self.0[0]);

        let bytes = &self.0[1..(1 + collection_name_len)];

        from_utf8(bytes).unwrap()
    }

    pub fn get_generation_id(&self) -> GenerationId<'a> {
        let collection_name_len = u8_to_usize(self.0[0]);

        let bytes = &self.0[(2 + collection_name_len)..];

        GenerationId::new_unchecked(bytes)
    }

    pub fn from_slice(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() < 2 {
            return Err(());
        }

        let collection_name_len = u8_to_usize(bytes[0]);

        if bytes.len() < 2 + collection_name_len {
            return Err(());
        }

        let utf8_validation = from_utf8(&bytes[1..(1 + collection_name_len)]);
        match utf8_validation {
            Err(_) => {
                return Err(());
            }
            _ => {}
        }

        let generation_id_len = u8_to_usize(bytes[1 + collection_name_len]);

        if bytes.len() != 2 + collection_name_len + generation_id_len {
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
