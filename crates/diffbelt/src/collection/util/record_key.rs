use std::fmt::{Debug, Formatter};
use std::iter;
use std::ops::Range;

use diffbelt_util_no_std::cast::{u32_to_usize, u8_to_usize};

use crate::common::constants::{
    MAX_COLLECTION_KEY_LENGTH, MAX_GENERATION_ID_LENGTH, MAX_PHANTOM_ID_LENGTH,
};
use crate::common::{CollectionKey, GenerationId, IsByteArray, PhantomId};
use crate::util::bytes::{read_u24, write_u24_be};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct RecordKey<'a> {
    value: &'a [u8],
}

impl<'a> RecordKey<'a> {
    pub fn new_unchecked(value: &'a [u8]) -> Self {
        Self { value }
    }
}

#[deprecated(note = "Use ParsedRecordKey")]
pub struct ParsedRecordKeyOld<'a> {
    pub collection_key: CollectionKey<'a>,
    pub generation_id: GenerationId<'a>,
    pub phantom_id: Option<PhantomId<'a>>,
}

pub struct ParsedRecordKey<'a> {
    bytes_inner: &'a [u8],
    ranges_inner: ParsedRecordKeyRanges,
}

impl PartialEq for ParsedRecordKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes_inner == other.bytes_inner
    }
}

impl<'a> ParsedRecordKey<'a> {
    pub fn new_unchecked(bytes: &'a [u8], ranges: ParsedRecordKeyRanges) -> Self {
        Self {
            bytes_inner: bytes,
            ranges_inner: ranges,
        }
    }

    pub fn new_on_vec(
        vec: &'a mut Vec<u8>,
        key: CollectionKey<'_>,
        generation_id: GenerationId<'_>,
        phantom_id: Option<PhantomId<'_>>,
    ) -> Result<Self, ()> {
        let key_bytes = key.get_byte_array();
        let generation_id_bytes = generation_id.get_byte_array();
        let phantom_id_bytes = phantom_id.unwrap_or(PhantomId::empty());
        let phantom_id_bytes = phantom_id_bytes.get_byte_array();

        if key_bytes.len() > MAX_COLLECTION_KEY_LENGTH
            || generation_id_bytes.len() > MAX_GENERATION_ID_LENGTH
            || phantom_id_bytes.len() > MAX_PHANTOM_ID_LENGTH
        {
            return Err(());
        }

        let len =
            1 + 3 + key_bytes.len() + 1 + generation_id_bytes.len() + 1 + phantom_id_bytes.len();

        vec.clear();
        vec.reserve(len);
        vec.extend(iter::repeat(0u8).take(len));

        write_record_key(
            &mut vec.as_mut_slice()[0..len],
            key_bytes,
            generation_id_bytes,
            phantom_id_bytes,
        );

        let mut offset = 4;
        let mut offset_to = offset + key_bytes.len();
        let collection_key = offset..offset_to;
        offset = offset_to + 1;
        offset_to = offset + generation_id_bytes.len();
        let generation_id = offset..offset_to;
        offset = offset_to + 1;
        offset_to = offset + phantom_id_bytes.len();
        let phantom_id = if offset < offset_to {
            Some(offset..offset_to)
        } else {
            None
        };

        let ranges = ParsedRecordKeyRanges {
            collection_key,
            generation_id,
            phantom_id,
        };

        Ok(Self {
            bytes_inner: &*vec,
            ranges_inner: ranges,
        })
    }

    pub fn as_record_key(&self) -> RecordKey<'a> {
        RecordKey {
            value: self.bytes_inner,
        }
    }

    pub fn collection_key(&self) -> CollectionKey<'a> {
        CollectionKey::new_unchecked(&self.bytes_inner[self.ranges_inner.collection_key.clone()])
    }

    pub fn generation_id(&self) -> GenerationId<'a> {
        GenerationId::new_unchecked(&self.bytes_inner[self.ranges_inner.generation_id.clone()])
    }

    pub fn phantom_id(&self) -> Option<PhantomId<'a>> {
        self.ranges_inner
            .phantom_id
            .as_ref()
            .map(|range| PhantomId::new_unchecked(&self.bytes_inner[range.clone()]))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes_inner
    }

    pub fn ranges(&self) -> &ParsedRecordKeyRanges {
        &self.ranges_inner
    }
}

impl Debug for ParsedRecordKey<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        () = f.write_fmt(format_args!(
            "ParsedRecordKey(CollectionKey = {:?}, GenerationId = {:?}, PhantomId = {:?})",
            self.collection_key(),
            self.generation_id(),
            self.phantom_id()
        ))?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ParsedRecordKeyRanges {
    pub collection_key: Range<usize>,
    pub generation_id: Range<usize>,
    pub phantom_id: Option<Range<usize>>,
}

impl Default for ParsedRecordKeyRanges {
    fn default() -> Self {
        Self {
            collection_key: 0..0,
            generation_id: 0..0,
            phantom_id: None,
        }
    }
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

    pub fn get_parsed(&self) -> ParsedRecordKeyOld<'_> {
        ParsedRecordKeyOld {
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

    pub fn get_phantom_id(&self) -> Option<PhantomId<'_>> {
        self.phantom_id
            .as_ref()
            .map(|x| PhantomId::new_unchecked(&self.bytes[x.clone()]))
    }

    pub fn to_owned_record_key(&self) -> OwnedRecordKey {
        OwnedRecordKey::new_unchecked(self.bytes.clone())
    }

    pub fn into_owned_record_key(self) -> OwnedRecordKey {
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

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
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

        let key_size = u32_to_usize(read_u24(bytes, 1));
        if rest_size < key_size {
            return false;
        }

        let mut offset = 4 + key_size;
        rest_size -= key_size;

        let generation_id_size = u8_to_usize(bytes[offset]);
        if rest_size < generation_id_size {
            return false;
        }

        offset += 1 + generation_id_size;
        rest_size -= generation_id_size;

        let phantom_id_size = u8_to_usize(bytes[offset]);
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
        let size = u32_to_usize(read_u24(self.value, 1));
        CollectionKey::new_unchecked(&self.value[4..(4 + size)])
    }

    pub fn get_generation_id(&self) -> GenerationId {
        let key_size = u32_to_usize(read_u24(self.value, 1));
        let mut offset = 4 + key_size;
        let size = u8_to_usize(self.value[offset]);
        offset += 1;
        GenerationId::new_unchecked(&self.value[offset..(offset + size)])
    }

    pub fn get_phantom_id(&self) -> Option<PhantomId> {
        let key_size = u32_to_usize(read_u24(self.value, 1));
        let mut offset = 4 + key_size;
        let generation_id_size = u8_to_usize(self.value[offset]);
        offset += 1 + generation_id_size;
        let size = u8_to_usize(self.value[offset]);
        offset += 1;
        PhantomId::new(&self.value[offset..(offset + size)])
    }

    pub fn parse_old(&self) -> ParsedRecordKeyOld<'a> {
        let (collection_key, generation_id, phantom_id) = self.parse_to_ranges();

        ParsedRecordKeyOld {
            collection_key: CollectionKey::new_unchecked(by_range(self.value, &collection_key)),
            generation_id: GenerationId::new_unchecked(by_range(self.value, &generation_id)),
            phantom_id: phantom_id
                .as_ref()
                .map(|range| PhantomId::new_unchecked(by_range(&self.value, range))),
        }
    }

    pub fn parse(&self) -> ParsedRecordKey<'a> {
        let (collection_key, generation_id, phantom_id) = self.parse_to_ranges();

        ParsedRecordKey {
            bytes_inner: &self.value,
            ranges_inner: ParsedRecordKeyRanges {
                collection_key,
                generation_id,
                phantom_id,
            },
        }
    }

    fn parse_to_ranges(&self) -> (Range<usize>, Range<usize>, Option<Range<usize>>) {
        let key_size = u32_to_usize(read_u24(self.value, 1));
        let collection_key = 4..(4 + key_size);

        let mut offset = 4 + key_size;

        let generation_id_size = u8_to_usize(self.value[offset]);
        offset += 1;
        let generation_id = offset..(offset + generation_id_size);

        offset += generation_id_size;
        let phantom_id_size = u8_to_usize(self.value[offset]);
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

fn write_record_key(
    value: &mut [u8],
    key_bytes: &[u8],
    generation_id_bytes: &[u8],
    phantom_id_bytes: &[u8],
) {
    // reserved for the future, if we will want to change keys format
    value[0] = 0;

    write_u24_be(value, 1, key_bytes.len() as u32);

    let mut offset = 4usize;

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
        (&mut value[offset..(offset + phantom_id_bytes.len())]).copy_from_slice(phantom_id_bytes);
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
            0u8;
            1 + 3
                + key_bytes.len()
                + 1
                + generation_id_bytes.len()
                + 1
                + phantom_id_bytes.len()
        ]
        .into_boxed_slice();

        write_record_key(&mut value, key_bytes, generation_id_bytes, phantom_id_bytes);

        Ok(OwnedRecordKey { value })
    }

    fn new_unchecked(value: Box<[u8]>) -> Self {
        Self { value }
    }

    pub fn from_owned_parsed_record_key(parsed: OwnedParsedRecordKey) -> Self {
        Self {
            value: parsed.bytes,
        }
    }

    pub fn get_collection_key_bytes_mut(&mut self) -> &mut [u8] {
        let size = u32_to_usize(read_u24(&self.value, 1));
        &mut self.value[4..(4 + size)]
    }

    pub fn as_ref(&self) -> RecordKey {
        self.into()
    }
}

impl From<ParsedRecordKeyOld<'_>> for OwnedRecordKey {
    fn from(value: ParsedRecordKeyOld) -> Self {
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

        assert_eq!(actual_key, key.get_byte_array());
        assert_eq!(actual_generation_id, generation_id.get_byte_array());
        assert_eq!(actual_phantom_id, Some(phantom_id.as_ref()));
    }
}
