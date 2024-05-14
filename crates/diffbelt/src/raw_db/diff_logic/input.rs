use enum_as_inner::EnumAsInner;

use crate::collection::util::record_key::ParsedRecordKey;
use crate::common::CollectionKey;

#[derive(EnumAsInner)]
pub enum DiffLogicInput<'a> {
    Init,
    NextChangedKey(Option<CollectionKey<'a>>),
    NextCursorKey(Option<ParsedRecordKey<'a>>),
    NextCursorAndChangedKey((Option<ParsedRecordKey<'a>>, Option<CollectionKey<'a>>)),
}
