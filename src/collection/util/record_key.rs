use crate::collection::util::record_flags::RecordFlags;
use crate::common::{
    CollectionKey, CollectionKeyRef, GenerationId, GenerationIdRef, IsByteArray, PhantomId,
    PhantomIdRef,
};
use crate::util::bytes::{read_u24, write_u24};
use std::ops::Deref;

pub struct RecordKey<'a> {
    pub value: &'a [u8],
}

impl<'a> From<&'a OwnedRecordKey> for RecordKey<'a> {
    fn from(record_key: &OwnedRecordKey) -> RecordKey {
        RecordKey {
            value: &record_key.value,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct OwnedRecordKey {
    pub value: Box<[u8]>,
}

impl Deref for OwnedRecordKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/*
    1 -- reserved byte
    3 -- size of key
    1 -- size of generationId
    1 -- size of phantomId
*/
const MIN_RECORD_KEY_LENGTH: usize = 1 + 3 + 1 + 1;
const MAX_KEY_LENGTH: usize = (2 as usize).pow(24) - 1;
const MAX_GENERATION_ID_LENGTH: usize = 255;
const MAX_PHANTOM_ID_LENGTH: usize = 255;

impl<'a> RecordKey<'a> {
    pub fn validate(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() < MIN_RECORD_KEY_LENGTH {
            return Err(());
        }

        let mut rest_size = bytes.len() - MIN_RECORD_KEY_LENGTH;

        let key_size = read_u24(bytes, 1) as usize;
        if rest_size < key_size {
            return Err(());
        }

        let mut offset = 4 + key_size;
        rest_size -= key_size;

        let generation_id_size = bytes[offset] as usize;
        if rest_size < generation_id_size {
            return Err(());
        }

        offset += 1 + generation_id_size;
        rest_size -= generation_id_size;

        let phantom_id_size = bytes[offset] as usize;
        if rest_size != phantom_id_size {
            return Err(());
        }

        Ok(Self { value: bytes })
    }

    pub fn get_key(&self) -> CollectionKeyRef {
        let size = read_u24(self.value, 1) as usize;
        CollectionKeyRef(&self.value[4..(4 + size)])
    }

    pub fn get_generation_id(&self) -> GenerationIdRef {
        let key_size = read_u24(self.value, 1) as usize;
        let mut offset = 4 + key_size;
        let size = self.value[offset] as usize;
        offset += 1;
        GenerationIdRef(&self.value[offset..(offset + size)])
    }

    pub fn get_phantom_id(&self) -> PhantomIdRef {
        let key_size = read_u24(self.value, 1) as usize;
        let mut offset = 4 + key_size;
        let generation_id_size = self.value[offset] as usize;
        offset += 1 + generation_id_size;
        let size = self.value[offset] as usize;
        offset += 1;
        PhantomIdRef(&self.value[offset..(offset + size)])
    }

    pub fn to_owned(&self) -> OwnedRecordKey {
        OwnedRecordKey {
            value: self.value.into(),
        }
    }
}

impl OwnedRecordKey {
    pub fn new<'a>(
        key: CollectionKeyRef<'a>,
        generation_id: GenerationIdRef<'a>,
        phantom_id: PhantomIdRef<'a>,
    ) -> Result<OwnedRecordKey, ()> {
        let key_bytes = key.get_byte_array();
        let generation_id_bytes = generation_id.get_byte_array();
        let phantom_id_bytes = phantom_id.get_byte_array();

        if key_bytes.len() > MAX_KEY_LENGTH
            || generation_id_bytes.len() > MAX_GENERATION_ID_LENGTH
            || phantom_id_bytes.len() > MAX_PHANTOM_ID_LENGTH
        {
            return Err(());
        }

        let mut value = vec![
            0 as u8;
            1 + 3
                + key_bytes.len()
                + 1
                + generation_id_bytes.len()
                + 1
                + phantom_id_bytes.len()
        ]
        .into_boxed_slice();

        // reserved for the future, if we will want to change keys format
        value[0] = 0;

        write_u24(&mut value, 1, key_bytes.len() as u32);

        let mut offset = 4 as usize;

        {
            (&mut value[offset..(offset + key_bytes.len())]).copy_from_slice(key_bytes);
            offset += key_bytes.len();
        }

        value[offset] = generation_id_bytes.len() as u8;
        offset += 1;

        {
            (&mut value[offset..(offset + generation_id_bytes.len())])
                .copy_from_slice(generation_id_bytes);
            offset += generation_id_bytes.len();
        }

        value[offset] = phantom_id_bytes.len() as u8;
        offset += 1;

        {
            (&mut value[offset..(offset + phantom_id_bytes.len())])
                .copy_from_slice(phantom_id_bytes);
        }

        Ok(OwnedRecordKey { value })
    }

    pub fn get_collection_key_bytes_mut(&mut self) -> &mut [u8] {
        let size = read_u24(&self.value, 1) as usize;
        &mut self.value[4..(4 + size)]
    }

    pub fn as_ref(&self) -> RecordKey {
        self.into()
    }
}

#[test]
fn test_create_record_key() {
    let key = CollectionKey(vec![1, 2, 3, 4, 5, 6, 7].into_boxed_slice());
    let generation_id = GenerationId(vec![8, 0, 2].into_boxed_slice());
    let phantom_id = PhantomId(vec![8, 2, 5, 1, 1].into_boxed_slice());

    let record_key = OwnedRecordKey::new(&key, &generation_id, &phantom_id);
    assert_eq!(record_key.is_ok(), true);

    let record_key = record_key.unwrap();
    let record_key = record_key.as_ref();

    assert_eq!(RecordKey::validate(record_key.value).is_ok(), true);

    let actual_key = record_key.get_key();
    let actual_key = actual_key.get_byte_array();

    let actual_generation_id = record_key.get_generation_id();
    let actual_generation_id = actual_generation_id.get_byte_array();

    let actual_phantom_id = record_key.get_phantom_id();
    let actual_phantom_id = actual_phantom_id.get_byte_array();

    assert_eq!(actual_key, key.get_byte_array());
    assert_eq!(actual_generation_id, generation_id.get_byte_array());
    assert_eq!(actual_phantom_id, phantom_id.get_byte_array());
}
