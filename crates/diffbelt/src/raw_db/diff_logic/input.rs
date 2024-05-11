use crate::collection::util::record_key::RecordKey;
use enum_as_inner::EnumAsInner;

#[derive(EnumAsInner)]
pub enum DiffLogicInput<'a> {
    NextChangedKey(Option<RecordKey<'a>>),
}
