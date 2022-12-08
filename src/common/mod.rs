use std::ops::{Deref, DerefMut};

pub mod util;

pub struct CollectionKey(pub Box<[u8]>);
pub struct CollectionKeyRef<'a>(pub &'a [u8]);

pub struct CollectionValue(pub Box<[u8]>);
pub struct CollectionValueRef<'a>(pub &'a [u8]);

#[derive(Clone)]
pub struct GenerationId(pub Box<[u8]>);
pub struct GenerationIdRef<'a>(pub &'a [u8]);

pub struct PhantomId(pub Box<[u8]>);
pub struct PhantomIdRef<'a>(pub &'a [u8]);

pub struct KeyValueUpdate {
    key: CollectionKey,
    value: Option<CollectionValue>,
    phantom_id: Option<PhantomId>,
    if_not_present: bool,
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
impl Deref for GenerationIdRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
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
impl Deref for PhantomId {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Deref for PhantomIdRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
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
