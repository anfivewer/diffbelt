use crate::aggregate::state::TargetKeyReducingChunkId;
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
}

#[derive(Debug)]
pub struct MapContext {
    pub bytes_to_free: usize,
}

#[derive(Debug)]
pub struct TargetRecordContext {
    pub target_key: Rc<[u8]>,
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
