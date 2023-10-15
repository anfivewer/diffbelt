use crate::common::{CollectionKey, GenerationId, IsByteArray, IsByteArrayMut};
use crate::util::bytes::{read_u24, write_u24};
use diffbelt_util::cast::{u32_to_usize, u8_to_usize};
use std::ops::Deref;

pub struct GenerationKey<'a> {
    pub value: &'a [u8],
}

impl<'a> From<&'a OwnedGenerationKey> for GenerationKey<'a> {
    fn from(record_key: &OwnedGenerationKey) -> GenerationKey {
        GenerationKey {
            value: &record_key.value,
        }
    }
}
impl From<OwnedGenerationKey> for Box<[u8]> {
    fn from(key: OwnedGenerationKey) -> Self {
        key.value
    }
}
impl Deref for OwnedGenerationKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Clone)]
pub struct OwnedGenerationKey {
    pub value: Box<[u8]>,
}

impl IsByteArray for OwnedGenerationKey {
    fn get_byte_array(&self) -> &[u8] {
        &self.value
    }
}
impl IsByteArrayMut<'_> for OwnedGenerationKey {
    fn get_byte_array_mut(&mut self) -> &mut [u8] {
        &mut self.value
    }
}

/*
    1 -- reserved byte
    1 -- size of generationId
    3 -- size of key
*/
const MIN_GENERATION_KEY_LENGTH: usize = 1 + 1 + 3;

impl<'a> GenerationKey<'a> {
    pub fn validate(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() < MIN_GENERATION_KEY_LENGTH || bytes[0] != 0 {
            return Err(());
        }

        let generation_id_size = u8_to_usize(bytes[1]);
        if bytes.len() - 2 < generation_id_size + 3 {
            return Err(());
        }

        let key_size = u32_to_usize(read_u24(bytes, 2 + generation_id_size));
        if bytes.len() - 2 - 3 - generation_id_size != key_size {
            return Err(());
        }

        Ok(Self { value: bytes })
    }

    pub fn get_collection_key(&self) -> CollectionKey {
        let generation_id_size = u8_to_usize(self.value[1]);
        let mut offset = 2 + generation_id_size;
        let size = u32_to_usize(read_u24(self.value, offset));
        offset += 3;
        CollectionKey::new_unchecked(&self.value[offset..(offset + size)])
    }

    pub fn get_generation_id(&self) -> GenerationId {
        let size = u8_to_usize(self.value[1]);
        GenerationId::new_unchecked(&self.value[2..(2 + size)])
    }
}

const MAX_KEY_LENGTH: usize = (2usize).pow(24) - 1;
const MAX_GENERATION_ID_LENGTH: usize = 255;

impl OwnedGenerationKey {
    pub fn new<'a>(
        generation_id: GenerationId<'a>,
        key: CollectionKey<'a>,
    ) -> Result<OwnedGenerationKey, ()> {
        let key_bytes = key.get_byte_array();
        let generation_id_bytes = generation_id.get_byte_array();

        if key_bytes.len() > MAX_KEY_LENGTH || generation_id_bytes.len() > MAX_GENERATION_ID_LENGTH
        {
            return Err(());
        }

        let mut value =
            vec![0u8; 1 + 3 + key_bytes.len() + 1 + generation_id_bytes.len()].into_boxed_slice();

        // reserved for the future
        value[0] = 0;

        let mut offset = 1;

        value[offset] = generation_id_bytes.len() as u8;
        offset += 1;
        {
            (&mut value[offset..(offset + generation_id_bytes.len())])
                .copy_from_slice(generation_id_bytes);
            offset += generation_id_bytes.len();
        }

        write_u24(&mut value, offset, key_bytes.len() as u32);
        offset += 3;
        {
            (&mut value[offset..(offset + key_bytes.len())]).copy_from_slice(key_bytes);
        }

        Ok(OwnedGenerationKey { value })
    }

    pub fn as_ref(&self) -> GenerationKey {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
    use crate::common::{CollectionKey, IsByteArray, OwnedCollectionKey, OwnedGenerationId};
    use std::ops::Deref;

    #[test]
    fn test_create_generation_key() {
        let key =
            OwnedCollectionKey::from_boxed_slice(vec![1, 2, 3, 4, 5, 6, 7].into_boxed_slice())
                .unwrap();
        let generation_id =
            OwnedGenerationId::from_boxed_slice(vec![8, 0, 2].into_boxed_slice()).unwrap();

        let generation_key = OwnedGenerationKey::new(generation_id.as_ref(), key.as_ref());
        assert_eq!(generation_key.is_ok(), true);

        let generation_key = generation_key.unwrap();
        let generation_key = generation_key.as_ref();

        assert_eq!(GenerationKey::validate(generation_key.value).is_ok(), true);

        let actual_key = generation_key.get_collection_key();
        let actual_key = actual_key.get_byte_array();

        let actual_generation_id = generation_key.get_generation_id();
        let actual_generation_id = actual_generation_id.get_byte_array();

        assert_eq!(actual_key, key.get_byte_array());
        assert_eq!(actual_generation_id, generation_id.get_byte_array());
    }

    #[test]
    fn empty_key_generation_key_test() {
        let generation_id =
            OwnedGenerationId::from_boxed_slice(vec![1, 2, 3, 4, 5, 6, 7, 8].into_boxed_slice())
                .unwrap();

        let generation_key =
            OwnedGenerationKey::new(generation_id.as_ref(), CollectionKey::empty()).unwrap();

        assert_eq!(
            generation_key.value.deref(),
            &[0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0]
        );
    }

    #[test]
    fn single_byte_key_generation_key_test() {
        let key = OwnedCollectionKey::from_boxed_slice(vec![42].into_boxed_slice()).unwrap();

        let generation_id =
            OwnedGenerationId::from_boxed_slice(vec![1, 2, 3, 4, 5, 6, 7, 8].into_boxed_slice())
                .unwrap();

        let generation_key = OwnedGenerationKey::new(generation_id.as_ref(), key.as_ref()).unwrap();

        assert_eq!(
            generation_key.value.deref(),
            &[0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 1, 42]
        );
    }
}
