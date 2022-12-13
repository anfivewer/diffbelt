
use crate::collection::util::record_flags::RecordFlags;
use crate::util::bytes::increment;
use std::cmp::Ordering;


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OwnedCollectionKey(pub Box<[u8]>);
pub struct CollectionKey<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct OwnedCollectionValue(Box<[u8]>);
pub struct CollectionValue<'a>(pub &'a [u8]);

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct OwnedGenerationId(pub Box<[u8]>);
#[derive(Copy, Clone)]
pub struct GenerationId<'a>(pub &'a [u8]);

pub struct OwnedPhantomId(pub Box<[u8]>);
#[derive(Copy, Clone)]
pub struct PhantomId<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct KeyValue {
    pub key: OwnedCollectionKey,
    pub value: OwnedCollectionValue,
}

pub struct KeyValueUpdate {
    pub key: OwnedCollectionKey,
    pub value: Option<OwnedCollectionValue>,
    pub if_not_present: bool,
}

impl OwnedGenerationId {
    pub fn increment(&mut self) {
        increment(&mut self.0);
    }
    pub fn as_ref(&self) -> GenerationId<'_> {
        GenerationId(&self.0)
    }
    pub fn replace(&mut self, other: OwnedGenerationId) {
        self.0 = other.0
    }
}
impl GenerationId<'_> {
    pub fn to_owned(&self) -> OwnedGenerationId {
        OwnedGenerationId(self.0.into())
    }
}

impl IsByteArray for OwnedGenerationId {
    fn get_byte_array(&self) -> &[u8] {
        &self.0
    }
}
impl IsByteArray for GenerationId<'_> {
    fn get_byte_array(&self) -> &[u8] {
        self.0
    }
}
impl IsByteArrayMut<'_> for OwnedGenerationId {
    fn get_byte_array_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl PartialEq for GenerationId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl PartialOrd for GenerationId<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(other.0))
    }
}
impl<'a> From<GenerationId<'a>> for &'a [u8] {
    fn from(gen: GenerationId<'a>) -> Self {
        gen.0
    }
}

impl OwnedCollectionKey {
    pub fn empty() -> Self {
        Self(vec![].into_boxed_slice())
    }
    pub fn as_ref(&self) -> CollectionKey<'_> {
        CollectionKey(&self.0)
    }
}
impl CollectionKey<'_> {
    pub fn empty() -> Self {
        Self(b"")
    }
    pub fn to_owned(&self) -> OwnedCollectionKey {
        OwnedCollectionKey(self.0.into())
    }
}
impl PartialEq for CollectionKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl IsByteArray for OwnedCollectionKey {
    fn get_byte_array(&self) -> &[u8] {
        &self.0
    }
}
impl IsByteArray for CollectionKey<'_> {
    fn get_byte_array(&self) -> &[u8] {
        self.0
    }
}
impl IsByteArrayMut<'_> for OwnedCollectionKey {
    fn get_byte_array_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl OwnedCollectionValue {
    pub fn new(bytes: &[u8]) -> Self {
        let mut vec = Vec::with_capacity(bytes.len() + 1);
        vec.push(RecordFlags::new().get_byte());
        vec.extend_from_slice(bytes);
        Self(vec.into_boxed_slice())
    }
    pub fn new_flags(bytes: &[u8], flags: RecordFlags) -> Self {
        let mut vec = Vec::with_capacity(bytes.len() + 1);
        vec.push(flags.get_byte());
        vec.extend_from_slice(bytes);
        Self(vec.into_boxed_slice())
    }
    pub fn from_boxed_slice(bytes: Box<[u8]>) -> Self {
        Self(bytes)
    }
    pub fn as_ref(&self) -> CollectionValue<'_> {
        CollectionValue(&self.0)
    }
}
impl CollectionValue<'_> {
    pub fn to_owned(&self) -> OwnedCollectionValue {
        OwnedCollectionValue(self.0.into())
    }
}

impl IsByteArray for OwnedCollectionValue {
    fn get_byte_array(&self) -> &[u8] {
        &self.0
    }
}
impl IsByteArray for CollectionValue<'_> {
    fn get_byte_array(&self) -> &[u8] {
        self.0
    }
}
impl IsByteArrayMut<'_> for OwnedCollectionValue {
    fn get_byte_array_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl OwnedPhantomId {
    pub fn empty() -> Self {
        Self(vec![].into_boxed_slice())
    }
    pub fn as_ref(&self) -> PhantomId<'_> {
        PhantomId(&self.0)
    }
    pub fn or_empty_as_ref(opt: &Option<Self>) -> PhantomId<'_> {
        match opt {
            Some(id) => id.as_ref(),
            None => PhantomId(b""),
        }
    }
}
impl PhantomId<'_> {
    pub fn empty() -> Self {
        Self(b"")
    }
    pub fn or_empty(opt: &Option<Self>) -> Self {
        match opt {
            Some(id) => Self(id.0),
            None => Self(b""),
        }
    }
}

impl IsByteArray for OwnedPhantomId {
    fn get_byte_array(&self) -> &[u8] {
        &self.0
    }
}
impl IsByteArray for PhantomId<'_> {
    fn get_byte_array(&self) -> &[u8] {
        self.0
    }
}
impl IsByteArrayMut<'_> for OwnedPhantomId {
    fn get_byte_array_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl PartialEq for PhantomId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub trait IsByteArray {
    fn get_byte_array(&self) -> &[u8];
}

pub trait IsByteArrayMut<'a> {
    fn get_byte_array_mut(&'a mut self) -> &'a mut [u8];
}

pub struct NeverEq;

unsafe impl Send for NeverEq {}

impl PartialEq for NeverEq {
    fn eq(&self, _: &Self) -> bool {
        false
    }

    fn ne(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for NeverEq {}

#[cfg(test)]
mod tests {
    use crate::common::NeverEq;

    #[test]
    pub fn never_eq_is_never_eq() {
        assert!(NeverEq != NeverEq);
        assert_eq!(NeverEq == NeverEq, false);
    }
}
