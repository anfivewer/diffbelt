use crate::collection::util::record_flags::RecordFlags;
use crate::util::bytes::increment;
use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};

pub mod util;

// TODO: remove this Defer implementations, prefer own methods
// TODO: rename "*Key" to "Owned*Key", rename "*KeyRef" to "*Key"

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CollectionKey(pub Box<[u8]>);
pub struct CollectionKeyRef<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct CollectionValue(Box<[u8]>);
pub struct CollectionValueRef<'a>(pub &'a [u8]);

#[derive(Clone, Debug)]
pub struct GenerationId(pub Box<[u8]>);
#[derive(Copy, Clone)]
pub struct GenerationIdRef<'a>(pub &'a [u8]);

pub struct PhantomId(pub Box<[u8]>);
#[derive(Copy, Clone)]
pub struct PhantomIdRef<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct KeyValue {
    pub key: CollectionKey,
    pub value: CollectionValue,
}

pub struct KeyValueUpdate {
    pub key: CollectionKey,
    pub value: Option<CollectionValue>,
    pub if_not_present: bool,
}

impl From<GenerationId> for Box<[u8]> {
    fn from(generation_id: GenerationId) -> Self {
        generation_id.0
    }
}
impl<'a> From<&'a GenerationId> for &'a [u8] {
    fn from(generation_id: &'a GenerationId) -> Self {
        &generation_id.0
    }
}
impl Deref for GenerationId {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl GenerationId {
    pub fn increment(&mut self) {
        increment(&mut self.0);
    }
    pub fn as_ref(&self) -> GenerationIdRef<'_> {
        GenerationIdRef(&self.0)
    }
}
impl Deref for GenerationIdRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl GenerationIdRef<'_> {
    pub fn to_owned(&self) -> GenerationId {
        GenerationId(self.0.into())
    }
}
impl PartialEq for GenerationIdRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl PartialOrd for GenerationIdRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(other.0))
    }
}
impl<'a> From<GenerationIdRef<'a>> for &'a [u8] {
    fn from(gen: GenerationIdRef<'a>) -> Self {
        gen.0
    }
}

impl Deref for CollectionKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Deref for CollectionKeyRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl CollectionKey {
    pub fn empty() -> Self {
        Self(vec![].into_boxed_slice())
    }
    pub fn as_ref(&self) -> CollectionKeyRef<'_> {
        CollectionKeyRef(&self.0)
    }
}
impl CollectionKeyRef<'_> {
    pub fn to_owned(&self) -> CollectionKey {
        CollectionKey(self.0.into())
    }
}
impl PartialEq for CollectionKeyRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl CollectionValue {
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
    pub fn as_ref(&self) -> CollectionValueRef<'_> {
        CollectionValueRef(&self.0)
    }
}
impl CollectionValueRef<'_> {
    pub fn to_owned(&self) -> CollectionValue {
        CollectionValue(self.0.into())
    }
}
impl Deref for CollectionValue {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Deref for CollectionValueRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Deref for PhantomId {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl PhantomId {
    pub fn empty() -> Self {
        Self(vec![].into_boxed_slice())
    }
    pub fn as_ref(&self) -> PhantomIdRef<'_> {
        PhantomIdRef(&self.0)
    }
    pub fn or_empty_as_ref(opt: &Option<Self>) -> PhantomIdRef<'_> {
        match opt {
            Some(id) => id.as_ref(),
            None => PhantomIdRef(b""),
        }
    }
}
impl PhantomIdRef<'_> {
    pub fn or_empty(opt: &Option<Self>) -> Self {
        match opt {
            Some(id) => PhantomIdRef(id.0),
            None => PhantomIdRef(b""),
        }
    }
}
impl Deref for PhantomIdRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl PartialEq for PhantomIdRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl DerefMut for GenerationId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait IsByteArray {
    fn get_byte_array(&self) -> &[u8];
}

pub trait IsByteArrayMut<'a> {
    fn get_byte_array_mut(&'a mut self) -> &'a mut [u8];
}

impl<'a, T: Deref<Target = [u8]>> IsByteArray for T {
    fn get_byte_array(&self) -> &[u8] {
        self
    }
}

impl<'a, T: DerefMut<Target = [u8]>> IsByteArrayMut<'a> for T {
    fn get_byte_array_mut(&'a mut self) -> &'a mut [u8] {
        self
    }
}
