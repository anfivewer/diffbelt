use crate::common::IsByteArray;

#[derive(Debug)]
pub struct OwnedReaderValue(Box<[u8]>);
pub struct ReaderValue<'a>(&'a [u8]);

impl OwnedReaderValue {
    pub fn new(collection_name: Option<&str>, generation_id: Option<&[u8]>) -> Result<Self, ()> {
        let collection_name: &[u8] = collection_name.map(|name| name.as_bytes()).unwrap_or(b"");
        let generation_id: &[u8] = generation_id.unwrap_or(b"");

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
    pub fn get_collection_name(&self) -> Result<&str, ()> {
        let collection_name_len = self.0[0] as usize;

        let bytes = &self.0[1..(1 + collection_name_len)];

        std::str::from_utf8(bytes).or(Err(()))
    }

    pub fn from_slice(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() < 2 {
            return Err(());
        }

        let collection_name_len = bytes[0] as usize;

        if bytes.len() < 2 + collection_name_len {
            return Err(());
        }

        let generation_id_len = bytes[1 + collection_name_len] as usize;

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
