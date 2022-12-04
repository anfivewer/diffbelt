pub type CollectionKey = Vec<u8>;
pub type CollectionValue = Vec<u8>;
pub type GenerationId = Vec<u8>;
pub type PhantomId = Vec<u8>;

pub struct KeyValueUpdate {
    key: CollectionKey,
    value: Option<CollectionValue>,
    phantom_id: Option<PhantomId>,
    if_not_present: bool,
}
