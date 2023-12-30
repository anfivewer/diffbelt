use crate::aggregate::state::{TargetKeyMergingChunkId, TargetKeyReducingChunkId};
use crate::base::error::TransformError;
use enum_as_inner::EnumAsInner;
use enum_kinds::EnumKind;
use std::rc::Rc;

#[derive(Debug, EnumAsInner, EnumKind)]
#[enum_kind(HandlerContextKind)]
pub enum HandlerContext {
    None,
    Map(MapContext),
    TargetRecord(TargetRecordContext),
    Reducing(ReducingContext),
    Merging(MergingContext),
    Applying(ApplyingContext),
}

#[derive(Debug)]
pub struct MapContext {
    pub bytes_to_free: u64,
}

#[derive(Debug)]
pub struct TargetRecordContext {
    pub target_key: Rc<[u8]>,
}

#[derive(Debug)]
pub struct ReducingContext {
    pub target_key: Rc<[u8]>,
    pub chunk_id: TargetKeyReducingChunkId,
    pub prev_accumulator_data_bytes: u64,
    pub transferring_target_data_bytes: u64,
}

#[derive(Debug)]
pub struct MergingContext {
    pub target_key_rc: Rc<[u8]>,
    pub chunk_id: TargetKeyMergingChunkId,
    pub accumulators_total_data_bytes: u64,
}

#[derive(Debug)]
pub struct ApplyingContext {
    pub target_key: Rc<[u8]>,
    pub applying_bytes: u64,
}

pub trait HandlerContextMapError<T> {
    fn map_err_self_to_transform_err(
        self,
        expected: HandlerContextKind,
    ) -> Result<T, TransformError>;
}

impl<T> HandlerContextMapError<T> for Result<T, HandlerContext> {
    fn map_err_self_to_transform_err(
        self,
        expected: HandlerContextKind,
    ) -> Result<T, TransformError> {
        match self {
            Ok(x) => Ok(x),
            Err(err) => Err(TransformError::Unspecified(format!(
                "Invalid map state, expected {expected:?}, got {:?}",
                HandlerContextKind::from(err)
            ))),
        }
    }
}
