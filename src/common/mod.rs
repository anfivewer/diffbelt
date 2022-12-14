use crate::collection::util::existing_value_flags::ExistingValueFlags;
use crate::common::constants::{
    MAX_COLLECTION_KEY_LENGTH, MAX_GENERATION_ID_LENGTH, MAX_PHANTOM_ID_LENGTH,
};
use crate::util::bytes::increment;
use std::cmp::Ordering;

pub mod constants;
pub mod generation_id;
pub mod reader;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OwnedCollectionKey(Box<[u8]>);
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct CollectionKey<'a>(&'a [u8]);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct OwnedCollectionValue(Box<[u8]>);
pub struct CollectionValue<'a>(&'a [u8]);

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct OwnedGenerationId(Box<[u8]>);
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct GenerationId<'a>(&'a [u8]);

#[derive(Clone)]
pub struct OwnedPhantomId(Box<[u8]>);
#[derive(Copy, Clone)]
pub struct PhantomId<'a>(&'a [u8]);

#[derive(PartialEq, Eq, Debug)]
pub struct KeyValue {
    pub key: OwnedCollectionKey,
    pub value: OwnedCollectionValue,
}

#[derive(PartialEq, Eq, Debug)]
pub struct KeyValueDiff {
    pub key: OwnedCollectionKey,
    pub from_value: Option<OwnedCollectionValue>,
    pub intermediate_values: Vec<Option<OwnedCollectionValue>>,
    pub to_value: Option<OwnedCollectionValue>,
}

#[derive(Clone)]
pub struct KeyValueUpdate {
    pub key: OwnedCollectionKey,
    pub value: Option<OwnedCollectionValue>,
    pub if_not_present: bool,
}

impl OwnedGenerationId {
    pub fn from_boxed_slice(bytes: Box<[u8]>) -> Result<Self, ()> {
        if bytes.len() > MAX_GENERATION_ID_LENGTH {
            return Err(());
        }

        Ok(Self(bytes))
    }
    pub fn empty() -> Self {
        Self(Box::from([]))
    }
    pub fn zero_64bits() -> Self {
        Self(vec![0; 8].into_boxed_slice())
    }

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
impl<'a> GenerationId<'a> {
    pub fn new_unchecked(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
    pub fn validate(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() > MAX_GENERATION_ID_LENGTH {
            return Err(());
        }

        Ok(Self(bytes))
    }

    pub fn empty() -> Self {
        GenerationId(b"")
    }

    pub fn cmp_with_opt_as_infinity(&self, other: Option<Self>) -> Ordering {
        match other {
            Some(other) => self.0.cmp(other.0),
            None => Ordering::Less,
        }
    }

    pub fn less_or_equal_with_opt_or(&self, other: Option<Self>, default: bool) -> bool {
        match other {
            Some(other) => {
                let cmp = self.0.cmp(other.0);
                cmp != Ordering::Greater
            }
            None => default,
        }
    }

    pub fn to_owned(&self) -> OwnedGenerationId {
        OwnedGenerationId(self.0.into())
    }
    pub fn to_opt_owned_if_empty(&self) -> Option<OwnedGenerationId> {
        if self.0.len() == 0 {
            None
        } else {
            Some(OwnedGenerationId(self.0.into()))
        }
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

impl<'a> From<GenerationId<'a>> for &'a [u8] {
    fn from(gen: GenerationId<'a>) -> Self {
        gen.0
    }
}

impl OwnedCollectionKey {
    pub fn from_boxed_slice(bytes: Box<[u8]>) -> Result<Self, ()> {
        if bytes.len() > MAX_COLLECTION_KEY_LENGTH {
            return Err(());
        }

        Ok(Self(bytes))
    }
    pub fn empty() -> Self {
        Self(vec![].into_boxed_slice())
    }
    pub fn as_ref(&self) -> CollectionKey<'_> {
        CollectionKey(&self.0)
    }
}
impl<'a> CollectionKey<'a> {
    pub fn new_unchecked(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
    pub fn validate(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() > MAX_COLLECTION_KEY_LENGTH {
            return Err(());
        }

        Ok(Self(bytes))
    }
    pub fn empty() -> Self {
        Self(b"")
    }
    pub fn or_empty(opt: &Option<Self>) -> Self {
        match opt {
            Some(id) => Self(id.0),
            None => Self(b""),
        }
    }
    pub fn to_owned(&self) -> OwnedCollectionKey {
        OwnedCollectionKey(self.0.into())
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
    // Value is prepended with single byte to allow to store empty strings
    pub fn new(bytes: &[u8]) -> Self {
        let mut vec = Vec::with_capacity(bytes.len() + 1);
        vec.push(ExistingValueFlags::new().get_byte());
        vec.extend_from_slice(bytes);
        Self(vec.into_boxed_slice())
    }
    pub fn new_flags(bytes: &[u8], flags: ExistingValueFlags) -> Self {
        let mut vec = Vec::with_capacity(bytes.len() + 1);
        vec.push(flags.get_byte());
        vec.extend_from_slice(bytes);
        Self(vec.into_boxed_slice())
    }
    pub fn from_boxed_slice(bytes: Box<[u8]>) -> Self {
        Self(bytes)
    }
    pub fn from_boxed_slice_opt(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.is_empty() {
            None
        } else {
            Some(Self(bytes))
        }
    }

    pub fn get_value(&self) -> &[u8] {
        &self.0[1..]
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_ref(&self) -> CollectionValue<'_> {
        CollectionValue(&self.0)
    }
}
impl CollectionValue<'_> {
    pub fn get_value(&self) -> &[u8] {
        &self.0[1..]
    }
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
    pub fn from_boxed_slice(bytes: Box<[u8]>) -> Result<Self, ()> {
        if bytes.len() > MAX_PHANTOM_ID_LENGTH {
            return Err(());
        }

        Ok(Self(bytes))
    }
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
impl<'a> PhantomId<'a> {
    pub fn new_unchecked(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
    pub fn validate(bytes: &'a [u8]) -> Result<Self, ()> {
        if bytes.len() > MAX_PHANTOM_ID_LENGTH {
            return Err(());
        }

        Ok(Self(bytes))
    }
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
