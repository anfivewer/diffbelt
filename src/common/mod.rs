pub mod util;

pub struct CollectionKey(pub Vec<u8>);
pub struct CollectionValue(pub Vec<u8>);
pub struct GenerationId(pub Vec<u8>);
pub struct PhantomId(pub Vec<u8>);

pub struct KeyValueUpdate {
    key: CollectionKey,
    value: Option<CollectionValue>,
    phantom_id: Option<PhantomId>,
    if_not_present: bool,
}

pub trait IsByteArray {
    fn get_byte_array(&self) -> &Vec<u8>;
}

impl IsByteArray for GenerationId {
    fn get_byte_array(&self) -> &Vec<u8> {
        return &self.0;
    }
}
