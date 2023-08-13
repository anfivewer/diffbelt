use crate::common::constants::{
    MAX_COLLECTION_KEY_LENGTH, MAX_GENERATION_ID_LENGTH, MAX_PHANTOM_ID_LENGTH,
};
use crate::common::{CollectionKey, GenerationId, IsByteArray, PhantomId};
use crate::util::bytes::{read_u24, write_u24};
use std::ops::Range;

#[derive(Clone, Eq, PartialEq)]
pub struct RecordKey<'a> {
    pub value: &'a [u8],
}
pub struct ParsedRecordKey<'a> {
    pub collection_key: CollectionKey<'a>,
    pub generation_id: GenerationId<'a>,
    pub phantom_id: Option<PhantomId<'a>>,
}

pub struct OwnedParsedRecordKey {
    bytes: Box<[u8]>,
    collection_key: Range<usize>,
    generation_id: Range<usize>,
    phantom_id: Option<Range<usize>>,
}

impl OwnedParsedRecordKey {
    pub fn from_boxed_slice(bytes: Box<[u8]>) -> Result<Self, ()> {
        let record_key = RecordKey::validate(&bytes)?;
        let (collection_key, generation_id, phantom_id) = record_key.parse_to_ranges();

        Ok(Self {
            bytes,
            collection_key,
            generation_id,
            phantom_id,
        })
    }

    pub fn from_owned_record_key(record_key: OwnedRecordKey) -> Self {
        let (collection_key, generation_id, phantom_id) = record_key.as_ref().parse_to_ranges();

        Self {
            bytes: record_key.value,
            collection_key,
            generation_id,
            phantom_id,
        }
    }

    pub fn empty() -> Self {
        Self {
            bytes: Box::new([]),
            collection_key: 0..0,
            generation_id: 0..0,
            phantom_id: None,
        }
    }

    pub fn get_parsed(&self) -> ParsedRecordKey<'_> {
        ParsedRecordKey {
            collection_key: CollectionKey::new_unchecked(by_range(
                &self.bytes,
                &self.collection_key,
            )),
            generation_id: GenerationId::new_unchecked(by_range(&self.bytes, &self.generation_id)),
            phantom_id: self
                .phantom_id
                .as_ref()
                .map(|range| PhantomId::new_unchecked(by_range(&self.bytes, range))),
        }
    }

    pub fn get_collection_key(&self) -> CollectionKey<'_> {
        CollectionKey::new_unchecked(by_range(&self.bytes, &self.collection_key))
    }

    pub fn to_owned_record_key(self) -> OwnedRecordKey {
        OwnedRecordKey::from_owned_parsed_record_key(self)
    }
}

#[inline]
fn by_range<'a>(bytes: &'a [u8], range: &Range<usize>) -> &'a [u8] {
    &bytes[range.clone()]
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

impl IsByteArray for RecordKey<'_> {
    fn get_byte_array(&self) -> &[u8] {
        self.value
    }
}

impl IsByteArray for OwnedRecordKey {
    fn get_byte_array(&self) -> &[u8] {
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

impl<'a> RecordKey<'a> {
    pub fn is_valid(bytes: &'a [u8]) -> bool {
        if bytes.len() < MIN_RECORD_KEY_LENGTH {
            return false;
        }

        let mut rest_size = bytes.len() - MIN_RECORD_KEY_LENGTH;

        let key_size = read_u24(bytes, 1) as usize;
        if rest_size < key_size {
            return false;
        }

        let mut offset = 4 + key_size;
        rest_size -= key_size;

        let generation_id_size = bytes[offset] as usize;
        if rest_size < generation_id_size {
            return false;
        }

        offset += 1 + generation_id_size;
        rest_size -= generation_id_size;

        let phantom_id_size = bytes[offset] as usize;
        if rest_size != phantom_id_size {
            return false;
        }

        true
    }

    pub fn validate(bytes: &'a [u8]) -> Result<Self, ()> {
        if !Self::is_valid(bytes) {
            return Err(());
        }

        Ok(Self { value: bytes })
    }

    pub fn get_collection_key(&self) -> CollectionKey {
        let size = read_u24(self.value, 1) as usize;
        CollectionKey::new_unchecked(&self.value[4..(4 + size)])
    }

    pub fn get_generation_id(&self) -> GenerationId {
        let key_size = read_u24(self.value, 1) as usize;
        let mut offset = 4 + key_size;
        let size = self.value[offset] as usize;
        offset += 1;
        GenerationId::new_unchecked(&self.value[offset..(offset + size)])
    }

    pub fn get_phantom_id(&self) -> PhantomId {
        let key_size = read_u24(self.value, 1) as usize;
        let mut offset = 4 + key_size;
        let generation_id_size = self.value[offset] as usize;
        offset += 1 + generation_id_size;
        let size = self.value[offset] as usize;
        offset += 1;
        PhantomId::new_unchecked(&self.value[offset..(offset + size)])
    }

    pub fn parse(&self) -> ParsedRecordKey<'_> {
        let (collection_key, generation_id, phantom_id) = self.parse_to_ranges();

        ParsedRecordKey {
            collection_key: CollectionKey::new_unchecked(by_range(self.value, &collection_key)),
            generation_id: GenerationId::new_unchecked(by_range(self.value, &generation_id)),
            phantom_id: phantom_id
                .as_ref()
                .map(|range| PhantomId::new_unchecked(by_range(&self.value, range))),
        }
    }

    fn parse_to_ranges(&self) -> (Range<usize>, Range<usize>, Option<Range<usize>>) {
        let key_size = read_u24(self.value, 1) as usize;
        let collection_key = 4..(4 + key_size);

        let mut offset = 4 + key_size;

        let generation_id_size = self.value[offset] as usize;
        offset += 1;
        let generation_id = offset..(offset + generation_id_size);

        offset += generation_id_size;
        let phantom_id_size = self.value[offset] as usize;
        offset += 1;

        let phantom_id_bytes = &self.value[offset..(offset + phantom_id_size)];
        let phantom_id = if phantom_id_bytes.len() == 0 {
            None
        } else {
            Some(offset..(offset + phantom_id_size))
        };

        (collection_key, generation_id, phantom_id)
    }

    pub fn to_owned(&self) -> OwnedRecordKey {
        OwnedRecordKey {
            value: self.value.into(),
        }
    }
}

impl OwnedRecordKey {
    pub fn new<'a>(
        key: CollectionKey<'a>,
        generation_id: GenerationId<'a>,
        phantom_id: PhantomId<'a>,
    ) -> Result<OwnedRecordKey, ()> {
        let key_bytes = key.get_byte_array();
        let generation_id_bytes = generation_id.get_byte_array();
        let phantom_id_bytes = phantom_id.get_byte_array();

        if key_bytes.len() > MAX_COLLECTION_KEY_LENGTH
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

    pub fn from_owned_parsed_record_key(parsed: OwnedParsedRecordKey) -> Self {
        Self {
            value: parsed.bytes,
        }
    }

    pub fn get_collection_key_bytes_mut(&mut self) -> &mut [u8] {
        let size = read_u24(&self.value, 1) as usize;
        &mut self.value[4..(4 + size)]
    }

    pub fn as_ref(&self) -> RecordKey {
        self.into()
    }
}

impl From<ParsedRecordKey<'_>> for OwnedRecordKey {
    fn from(value: ParsedRecordKey) -> Self {
        OwnedRecordKey::new(
            value.collection_key,
            value.generation_id,
            value.phantom_id.unwrap_or_else(|| PhantomId::empty()),
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::collection::util::record_key::{OwnedRecordKey, RecordKey};
    use crate::common::{IsByteArray, OwnedCollectionKey, OwnedGenerationId, OwnedPhantomId};

    #[test]
    fn test_create_record_key() {
        let key =
            OwnedCollectionKey::from_boxed_slice(vec![1, 2, 3, 4, 5, 6, 7].into_boxed_slice())
                .unwrap();
        let generation_id =
            OwnedGenerationId::from_boxed_slice(vec![8, 0, 2].into_boxed_slice()).unwrap();
        let phantom_id =
            OwnedPhantomId::from_boxed_slice(vec![8, 2, 5, 1, 1].into_boxed_slice()).unwrap();

        let record_key =
            OwnedRecordKey::new(key.as_ref(), generation_id.as_ref(), phantom_id.as_ref());
        assert_eq!(record_key.is_ok(), true);

        let record_key = record_key.unwrap();
        let record_key = record_key.as_ref();

        assert_eq!(RecordKey::validate(record_key.value).is_ok(), true);

        let actual_key = record_key.get_collection_key();
        let actual_key = actual_key.get_byte_array();

        let actual_generation_id = record_key.get_generation_id();
        let actual_generation_id = actual_generation_id.get_byte_array();

        let actual_phantom_id = record_key.get_phantom_id();
        let actual_phantom_id = actual_phantom_id.get_byte_array();

        assert_eq!(actual_key, key.get_byte_array());
        assert_eq!(actual_generation_id, generation_id.get_byte_array());
        assert_eq!(actual_phantom_id, phantom_id.get_byte_array());
    }
}
