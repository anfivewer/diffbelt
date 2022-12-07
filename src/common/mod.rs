pub mod util;

pub struct CollectionKey(pub Vec<u8>);
pub struct CollectionKeyRef<'a>(pub &'a [u8]);
pub struct CollectionValue(pub Vec<u8>);
pub struct CollectionValueRef<'a>(pub &'a [u8]);

#[derive(Clone)]
pub struct GenerationId(pub Vec<u8>);
pub struct GenerationIdRef<'a>(pub &'a [u8]);
pub struct PhantomId(pub Vec<u8>);
pub struct PhantomIdRef<'a>(pub &'a [u8]);

pub struct KeyValueUpdate {
    key: CollectionKey,
    value: Option<CollectionValue>,
    phantom_id: Option<PhantomId>,
    if_not_present: bool,
}

pub trait IsByteArray {
    fn get_byte_array(&self) -> &[u8];
}

pub trait IsByteArrayMut {
    fn get_byte_array_mut(&mut self) -> &mut [u8];
}

impl IsByteArray for CollectionKey {
    fn get_byte_array(&self) -> &[u8] {
        return &self.0;
    }
}

impl IsByteArray for CollectionKeyRef<'_> {
    fn get_byte_array(&self) -> &[u8] {
        return self.0;
    }
}

impl IsByteArray for GenerationId {
    fn get_byte_array(&self) -> &[u8] {
        return &self.0;
    }
}
impl IsByteArrayMut for GenerationId {
    fn get_byte_array_mut(&mut self) -> &mut [u8] {
        return &mut self.0;
    }
}
impl From<GenerationId> for Vec<u8> {
    fn from(generation_id: GenerationId) -> Self {
        generation_id.0
    }
}

impl IsByteArray for GenerationIdRef<'_> {
    fn get_byte_array(&self) -> &[u8] {
        return &self.0;
    }
}

impl IsByteArray for PhantomId {
    fn get_byte_array(&self) -> &[u8] {
        return &self.0;
    }
}

impl IsByteArray for PhantomIdRef<'_> {
    fn get_byte_array(&self) -> &[u8] {
        return &self.0;
    }
}
